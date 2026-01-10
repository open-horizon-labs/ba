//! ba - Simple task tracking for LLM sessions
//!
//! A spiritual fork of beads (bd), keeping the simplicity of v0.9.6
//! with added session-based claiming for multi-agent coordination.

use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const ISSUES_FILE: &str = "issues.jsonl";
const CONFIG_FILE: &str = "config.json";

// ─────────────────────────────────────────────────────────────────────────────
// Data Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Status {
    Open,
    InProgress,
    Closed,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Open => write!(f, "open"),
            Status::InProgress => write!(f, "in_progress"),
            Status::Closed => write!(f, "closed"),
        }
    }
}

// AIDEV-NOTE: Issue types are minimal by design. Only types that signal
// different work patterns exist - priority handles urgency, title describes
// the work. Types:
// - task: default, general work
// - epic: container for grouping related issues
// - refactor: improving existing code (no new behavior)
// - spike: research/investigation (may not produce code)
// Legacy types (bug, feature, chore) deserialize to Task for backwards compat.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum IssueType {
    Epic,
    Refactor,
    Spike,
    #[serde(other)]
    Task,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueType::Task => write!(f, "task"),
            IssueType::Epic => write!(f, "epic"),
            IssueType::Refactor => write!(f, "refactor"),
            IssueType::Spike => write!(f, "spike"),
        }
    }
}

impl std::str::FromStr for IssueType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "task" => Ok(IssueType::Task),
            "epic" => Ok(IssueType::Epic),
            "refactor" => Ok(IssueType::Refactor),
            "spike" => Ok(IssueType::Spike),
            _ => Err(format!("Unknown issue type: {} (valid: task, epic, refactor, spike)", s)),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// State Machine
// ─────────────────────────────────────────────────────────────────────────────

/// Transitions that can be applied to an issue.
/// Status is a side-effect of ownership transitions, not set directly.
#[derive(Debug, Clone)]
enum Transition {
    /// Take ownership: (Open|Closed) → InProgress
    Claim { session: String },
    /// Abandon work: InProgress → Open
    Release,
    /// Complete work: InProgress → Closed
    Finish,
    /// Close unclaimed issue: Open → Closed (escape hatch)
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Comment {
    author: String,
    text: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Issue {
    id: String,
    title: String,
    #[serde(default)]
    description: String,
    status: Status,
    #[serde(default = "default_priority")]
    priority: u8,
    issue_type: IssueType,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    comments: Vec<Comment>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    closed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    blocks: Vec<String>,
    #[serde(default)]
    blocked_by: Vec<String>,
}

impl Issue {
    /// Apply a state transition to this issue.
    /// Returns the previous session_id if relevant (for release/finish messages).
    fn apply(&mut self, transition: Transition) -> Result<Option<String>, String> {
        let now = Utc::now();

        match (&self.status, &self.session_id, transition) {
            // Claim: Open + unclaimed → InProgress
            (Status::Open, None, Transition::Claim { session }) => {
                self.session_id = Some(session);
                self.status = Status::InProgress;
                self.updated_at = now;
                Ok(None)
            }

            // Claim: Open + already claimed by same session
            (Status::Open, Some(existing), Transition::Claim { session }) if existing == &session => {
                Err(format!("{} already claimed by this session", self.id))
            }

            // Claim: Open + already claimed by different session
            (Status::Open, Some(existing), Transition::Claim { .. }) => {
                Err(format!("{} already claimed by session {}", self.id, existing))
            }

            // Claim: Closed → InProgress (reopen)
            (Status::Closed, _, Transition::Claim { session }) => {
                self.session_id = Some(session);
                self.status = Status::InProgress;
                self.closed_at = None;
                self.updated_at = now;
                Ok(None)
            }

            // Claim: InProgress + already claimed
            (Status::InProgress, Some(existing), Transition::Claim { session }) if existing == &session => {
                Err(format!("{} already claimed by this session", self.id))
            }

            (Status::InProgress, Some(existing), Transition::Claim { .. }) => {
                Err(format!("{} already claimed by session {}", self.id, existing))
            }

            // Release: InProgress + claimed → Open
            (Status::InProgress, Some(_), Transition::Release) => {
                let old_session = self.session_id.take();
                self.status = Status::Open;
                self.updated_at = now;
                Ok(old_session)
            }

            // Release: not claimed
            (_, None, Transition::Release) => {
                Err(format!("{} is not claimed", self.id))
            }

            // Release: not in progress (but claimed somehow - shouldn't happen)
            (_, Some(_), Transition::Release) => {
                Err(format!("{} is not in progress", self.id))
            }

            // Finish: InProgress + claimed → Closed
            (Status::InProgress, Some(_), Transition::Finish) => {
                let old_session = self.session_id.take();
                self.status = Status::Closed;
                self.closed_at = Some(now);
                self.updated_at = now;
                Ok(old_session)
            }

            // Finish: not claimed
            (_, None, Transition::Finish) => {
                Err(format!("{} is not claimed. Use 'close' for unclaimed issues.", self.id))
            }

            // Finish: already closed
            (Status::Closed, _, Transition::Finish) => {
                Err(format!("{} is already closed", self.id))
            }

            // Finish: open but not claimed (shouldn't have session)
            (Status::Open, Some(_), Transition::Finish) => {
                Err(format!("{} is open, not in progress", self.id))
            }

            // Close: Open + unclaimed → Closed (escape hatch)
            (Status::Open, None, Transition::Close) => {
                self.status = Status::Closed;
                self.closed_at = Some(now);
                self.updated_at = now;
                Ok(None)
            }

            // Close: already closed
            (Status::Closed, _, Transition::Close) => {
                Err(format!("{} is already closed", self.id))
            }

            // Close: claimed - must release first or use finish
            (_, Some(session), Transition::Close) => {
                Err(format!(
                    "{} is claimed by session {}. Use 'release' first, or 'finish' to complete.",
                    self.id, session
                ))
            }

            // Invalid states (InProgress without session shouldn't exist)
            (Status::InProgress, None, Transition::Claim { session }) => {
                // Treat as claimable - fix the inconsistent state
                self.session_id = Some(session);
                self.updated_at = now;
                Ok(None)
            }

            (Status::InProgress, None, Transition::Close) => {
                // InProgress but no owner - treat as closeable
                self.status = Status::Closed;
                self.closed_at = Some(now);
                self.updated_at = now;
                Ok(None)
            }
        }
    }
}

fn default_priority() -> u8 {
    2
}

// Beads import types - using Value for flexible parsing with clear errors
#[derive(Debug, Deserialize)]
struct BeadsDependency {
    issue_id: String,
    depends_on_id: String,
    #[serde(rename = "type")]
    dep_type: String,
}

#[derive(Debug, Deserialize)]
struct BeadsIssue {
    id: String,
    title: String,
    #[serde(default)]
    description: String,
    status: String,
    #[serde(default = "default_priority")]
    priority: u8,
    issue_type: String,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    closed_at: Option<String>,
    #[serde(default)]
    dependencies: Vec<BeadsDependency>,
}

#[derive(Debug)]
struct ImportError {
    line_num: usize,
    issue_id: Option<String>,
    field: String,
    message: String,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.issue_id {
            Some(id) => write!(f, "Line {}: Issue '{}' - {}: {}",
                self.line_num, id, self.field, self.message),
            None => write!(f, "Line {}: {}: {}",
                self.line_num, self.field, self.message),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    version: u8,
    prefix: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Store (in-memory + file operations)
// ─────────────────────────────────────────────────────────────────────────────

struct Store {
    config: Config,
    issues: HashMap<String, Issue>,
    ba_dir: PathBuf,
}

impl Store {
    fn load(ba_dir: &Path) -> Result<Self, String> {
        let config_path = ba_dir.join(CONFIG_FILE);
        let config: Config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse config: {}", e))?
        } else {
            return Err("Not initialized. Run 'ba init' first.".to_string());
        };

        let issues_path = ba_dir.join(ISSUES_FILE);
        let mut issues = HashMap::new();
        if issues_path.exists() {
            let file = File::open(&issues_path)
                .map_err(|e| format!("Failed to open issues file: {}", e))?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
                if line.trim().is_empty() {
                    continue;
                }
                let issue: Issue = serde_json::from_str(&line)
                    .map_err(|e| format!("Failed to parse issue: {}", e))?;
                issues.insert(issue.id.clone(), issue);
            }
        }

        Ok(Store {
            config,
            issues,
            ba_dir: ba_dir.to_path_buf(),
        })
    }

    fn save(&self) -> Result<(), String> {
        // Sort issues by ID for consistent output
        let mut sorted: Vec<_> = self.issues.values().collect();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));

        let issues_path = self.ba_dir.join(ISSUES_FILE);
        let tmp_path = self.ba_dir.join("issues.jsonl.tmp");

        let mut file = File::create(&tmp_path)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        for issue in sorted {
            let line = serde_json::to_string(issue)
                .map_err(|e| format!("Failed to serialize issue: {}", e))?;
            writeln!(file, "{}", line)
                .map_err(|e| format!("Failed to write issue: {}", e))?;
        }

        fs::rename(&tmp_path, &issues_path)
            .map_err(|e| format!("Failed to rename temp file: {}", e))?;

        Ok(())
    }

    fn generate_id(&self, title: &str, timestamp: &DateTime<Utc>) -> String {
        let input = format!("{}{}", title, timestamp.to_rfc3339());
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let hash = hasher.finalize();

        // Try sliding window: bytes 0-3, then 1-4, then 2-5, etc.
        // SHA256 gives 32 bytes, so we can slide up to 28 times
        for offset in 0..28 {
            let suffix: String = hash[offset..offset + 4]
                .iter()
                .map(|b| {
                    let idx = (b % 36) as usize;
                    if idx < 10 {
                        (b'0' + idx as u8) as char
                    } else {
                        (b'a' + (idx - 10) as u8) as char
                    }
                })
                .collect();

            let id = format!("{}-{}", self.config.prefix, suffix);
            if !self.issues.contains_key(&id) {
                return id;
            }
        }

        // Extremely unlikely fallback: append counter
        let mut counter = 0u32;
        loop {
            let id = format!("{}-{:04x}", self.config.prefix, counter);
            if !self.issues.contains_key(&id) {
                return id;
            }
            counter += 1;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CLI
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "ba")]
#[command(about = "Simple task tracking for LLM sessions")]
#[command(version)]
struct Cli {
    /// Data directory (default: .ba/)
    #[arg(long, default_value = ".ba")]
    dir: PathBuf,

    /// Output in JSON format
    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .ba/ directory
    Init,

    /// Create a new issue
    #[command(visible_alias = "add", visible_alias = "new")]
    Create {
        /// Issue title
        title: String,

        /// Issue type (bug, feature, task, epic, chore, refactor, spike)
        #[arg(short = 't', long, default_value = "task")]
        issue_type: String,

        /// Priority (0-4, 0 = highest)
        #[arg(short, long, default_value = "2")]
        priority: u8,

        /// Description
        #[arg(short, long, default_value = "")]
        description: String,
    },

    /// List issues
    List {
        /// Filter by status (open, in_progress, closed)
        #[arg(long)]
        status: Option<String>,

        /// Include closed issues
        #[arg(long)]
        all: bool,
    },

    /// Show issue details
    Show {
        /// Issue ID
        id: String,
    },

    /// Close an issue
    Close {
        /// Issue ID
        id: String,

        /// Reason for closing
        #[arg(long)]
        reason: Option<String>,
    },

    /// Add a blocking dependency (blocker blocks id)
    Block {
        /// Issue that is blocked
        id: String,
        /// Issue that blocks it
        blocker: String,
    },

    /// Remove a blocking dependency
    Unblock {
        /// Issue that was blocked
        id: String,
        /// Issue that was blocking it
        blocker: String,
    },

    /// Show dependency tree
    Tree {
        /// Root issue ID
        id: String,
    },

    /// Detect circular dependencies
    Cycles,

    /// Show issues ready to work on (open, not blocked)
    Ready,

    /// Claim an issue for a session
    Claim {
        /// Issue ID
        id: String,
        /// Session ID (caller provides their own)
        #[arg(long)]
        session: String,
    },

    /// Release a claimed issue (back to open)
    Release {
        /// Issue ID
        id: String,
    },

    /// Finish a claimed issue (release + close)
    Finish {
        /// Issue ID
        id: String,
    },

    /// Show issues claimed by a session
    Mine {
        /// Session ID
        #[arg(long)]
        session: String,
    },

    /// Add or remove a label
    Label {
        /// Issue ID
        id: String,
        /// Action: add or remove
        action: String,
        /// Label name
        label: String,
    },

    /// Set priority of an issue
    Priority {
        /// Issue ID
        id: String,
        /// New priority (0-4, 0 = highest)
        value: u8,
    },

    /// Add a comment to an issue
    Comment {
        /// Issue ID
        id: String,
        /// Comment text
        text: String,
        /// Author name
        #[arg(long, default_value = "anonymous")]
        author: String,
    },

    /// Import issues from beads (bd) export
    Import {
        /// Input file (beads JSONL export)
        file: PathBuf,
        /// Keep original IDs (default: generate new with ba prefix)
        #[arg(long)]
        keep_ids: bool,
    },

    /// Quick start guide for LLMs
    Quickstart,
}

// ─────────────────────────────────────────────────────────────────────────────
// Command Implementations
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_init(ac_dir: &Path) -> Result<(), String> {
    if ac_dir.exists() {
        return Err(format!("{} already exists", ac_dir.display()));
    }

    fs::create_dir_all(ac_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    // Generate prefix from current directory hash
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    let cwd_str = cwd.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(cwd_str.as_bytes());
    let hash = hasher.finalize();

    // Take first 2 chars as base36
    let prefix: String = hash[0..2]
        .iter()
        .map(|b| {
            let idx = (b % 36) as usize;
            if idx < 10 {
                (b'0' + idx as u8) as char
            } else {
                (b'a' + (idx - 10) as u8) as char
            }
        })
        .collect();

    let config = Config { version: 1, prefix };
    let config_path = ac_dir.join(CONFIG_FILE);
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&config_path, config_json)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // Create empty issues file
    let issues_path = ac_dir.join(ISSUES_FILE);
    File::create(&issues_path)
        .map_err(|e| format!("Failed to create issues file: {}", e))?;

    println!("Initialized {} with prefix '{}'", ac_dir.display(), config.prefix);
    Ok(())
}

fn cmd_create(
    store: &mut Store,
    title: String,
    issue_type: String,
    priority: u8,
    description: String,
    json_output: bool,
) -> Result<(), String> {
    let issue_type: IssueType = issue_type.parse()?;

    if priority > 4 {
        return Err("Priority must be 0-4".to_string());
    }

    let now = Utc::now();
    let id = store.generate_id(&title, &now);

    let issue = Issue {
        id: id.clone(),
        title,
        description,
        status: Status::Open,
        priority,
        issue_type,
        session_id: None,
        labels: vec![],
        comments: vec![],
        created_at: now,
        updated_at: now,
        closed_at: None,
        blocks: vec![],
        blocked_by: vec![],
    };

    store.issues.insert(id.clone(), issue.clone());
    store.save()?;

    if json_output {
        println!("{}", serde_json::to_string(&issue).unwrap());
    } else {
        println!("Created {}", id);
    }

    Ok(())
}

fn cmd_list(store: &Store, status_filter: Option<String>, all: bool, json_output: bool) -> Result<(), String> {
    let mut issues: Vec<_> = store.issues.values().collect();

    // Filter
    if let Some(status) = status_filter {
        let status = match status.as_str() {
            "open" => Status::Open,
            "in_progress" => Status::InProgress,
            "closed" => Status::Closed,
            _ => return Err(format!("Unknown status: {}", status)),
        };
        issues.retain(|i| i.status == status);
    } else if !all {
        issues.retain(|i| i.status != Status::Closed);
    }

    // Sort by priority, then by created_at
    issues.sort_by(|a, b| {
        a.priority.cmp(&b.priority).then_with(|| a.created_at.cmp(&b.created_at))
    });

    if json_output {
        println!("{}", serde_json::to_string(&issues).unwrap());
        return Ok(());
    }

    if issues.is_empty() {
        println!("No issues found.");
        return Ok(());
    }

    // Pretty print
    println!();
    println!(
        "  {:<8} {:>2}  {:<8} {:<12} {}",
        "ID", "P", "TYPE", "STATUS", "TITLE"
    );
    println!("  {}", "-".repeat(70));

    for issue in &issues {
        println!(
            "  {:<8} {:>2}  {:<8} {:<12} {}",
            issue.id,
            issue.priority,
            issue.issue_type,
            issue.status,
            truncate(&issue.title, 40)
        );
    }

    let open = issues.iter().filter(|i| i.status == Status::Open).count();
    let in_progress = issues.iter().filter(|i| i.status == Status::InProgress).count();
    let closed = issues.iter().filter(|i| i.status == Status::Closed).count();

    println!();
    println!("{} issues ({} open, {} in_progress, {} closed)", issues.len(), open, in_progress, closed);

    Ok(())
}

fn cmd_show(store: &Store, id: &str, json_output: bool) -> Result<(), String> {
    let issue = store.issues.get(id).ok_or_else(|| format!("Issue not found: {}", id))?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(issue).unwrap());
        return Ok(());
    }

    println!();
    println!("{}: {}", issue.id, issue.title);
    println!("{}", "-".repeat(60));
    println!("Status:   {:<16} Priority: P{}", issue.status, issue.priority);
    println!("Type:     {}", issue.issue_type);
    if let Some(ref session) = issue.session_id {
        println!("Session:  {}", session);
    }
    println!("Created:  {}", issue.created_at.format("%Y-%m-%d %H:%M"));
    println!("Updated:  {}", issue.updated_at.format("%Y-%m-%d %H:%M"));
    if let Some(closed_at) = issue.closed_at {
        println!("Closed:   {}", closed_at.format("%Y-%m-%d %H:%M"));
    }
    if !issue.description.is_empty() {
        println!();
        println!("Description:");
        println!("{}", issue.description);
    }
    if !issue.blocked_by.is_empty() {
        println!();
        println!("Blocked by: {}", issue.blocked_by.join(", "));
    }
    if !issue.blocks.is_empty() {
        println!("Blocks: {}", issue.blocks.join(", "));
    }
    if !issue.labels.is_empty() {
        println!();
        println!("Labels: {}", issue.labels.join(", "));
    }
    if !issue.comments.is_empty() {
        println!();
        println!("Comments ({}):", issue.comments.len());
        for comment in &issue.comments {
            println!("  [{}] {}: {}",
                comment.created_at.format("%Y-%m-%d %H:%M"),
                comment.author,
                comment.text
            );
        }
    }

    Ok(())
}

fn cmd_close(store: &mut Store, id: &str, _reason: Option<String>, json_output: bool) -> Result<(), String> {
    let issue = store.issues.get_mut(id).ok_or_else(|| format!("Issue not found: {}", id))?;

    issue.apply(Transition::Close)?;

    let issue_clone = issue.clone();
    store.save()?;

    if json_output {
        println!("{}", serde_json::to_string(&issue_clone).unwrap());
    } else {
        println!("Closed {}", id);
    }

    Ok(())
}

fn cmd_block(store: &mut Store, id: &str, blocker: &str, json_output: bool) -> Result<(), String> {
    if id == blocker {
        return Err("Issue cannot block itself".to_string());
    }

    // Verify both issues exist
    if !store.issues.contains_key(id) {
        return Err(format!("Issue not found: {}", id));
    }
    if !store.issues.contains_key(blocker) {
        return Err(format!("Issue not found: {}", blocker));
    }

    // Check if already blocked
    {
        let issue = store.issues.get(id).unwrap();
        if issue.blocked_by.contains(&blocker.to_string()) {
            return Err(format!("{} already blocked by {}", id, blocker));
        }
    }

    // Add bidirectional relationship
    let now = Utc::now();
    {
        let issue = store.issues.get_mut(id).unwrap();
        issue.blocked_by.push(blocker.to_string());
        issue.updated_at = now;
    }
    {
        let blocker_issue = store.issues.get_mut(blocker).unwrap();
        blocker_issue.blocks.push(id.to_string());
        blocker_issue.updated_at = now;
    }

    store.save()?;

    if json_output {
        println!(r#"{{"blocked":"{}","blocker":"{}"}}"#, id, blocker);
    } else {
        println!("{} now blocked by {}", id, blocker);
    }

    Ok(())
}

fn cmd_unblock(store: &mut Store, id: &str, blocker: &str, json_output: bool) -> Result<(), String> {
    // Verify both issues exist
    if !store.issues.contains_key(id) {
        return Err(format!("Issue not found: {}", id));
    }
    if !store.issues.contains_key(blocker) {
        return Err(format!("Issue not found: {}", blocker));
    }

    // Check if relationship exists
    {
        let issue = store.issues.get(id).unwrap();
        if !issue.blocked_by.contains(&blocker.to_string()) {
            return Err(format!("{} is not blocked by {}", id, blocker));
        }
    }

    // Remove bidirectional relationship
    let now = Utc::now();
    {
        let issue = store.issues.get_mut(id).unwrap();
        issue.blocked_by.retain(|b| b != blocker);
        issue.updated_at = now;
    }
    {
        let blocker_issue = store.issues.get_mut(blocker).unwrap();
        blocker_issue.blocks.retain(|b| b != id);
        blocker_issue.updated_at = now;
    }

    store.save()?;

    if json_output {
        println!(r#"{{"unblocked":"{}","was_blocker":"{}"}}"#, id, blocker);
    } else {
        println!("{} no longer blocked by {}", id, blocker);
    }

    Ok(())
}

fn cmd_tree(store: &Store, id: &str, json_output: bool) -> Result<(), String> {
    let issue = store.issues.get(id).ok_or_else(|| format!("Issue not found: {}", id))?;

    if json_output {
        // Build tree structure as JSON
        let tree = build_tree_json(store, id, &mut vec![]);
        println!("{}", serde_json::to_string_pretty(&tree).unwrap());
        return Ok(());
    }

    // Pretty print tree
    println!();
    print_tree_node(store, issue, "", true, true, &mut vec![]);

    Ok(())
}

fn build_tree_json(store: &Store, id: &str, visited: &mut Vec<String>) -> serde_json::Value {
    if visited.contains(&id.to_string()) {
        return serde_json::json!({"id": id, "cycle": true});
    }
    visited.push(id.to_string());

    let issue = match store.issues.get(id) {
        Some(i) => i,
        None => return serde_json::json!({"id": id, "missing": true}),
    };

    let children: Vec<_> = issue
        .blocked_by
        .iter()
        .map(|child_id| build_tree_json(store, child_id, visited))
        .collect();

    visited.pop();

    serde_json::json!({
        "id": issue.id,
        "title": issue.title,
        "status": issue.status,
        "blocked_by": children
    })
}

fn print_tree_node(store: &Store, issue: &Issue, prefix: &str, is_root: bool, is_last: bool, visited: &mut Vec<String>) {
    let status_tag = match issue.status {
        Status::Open => "[OPEN]",
        Status::InProgress => "[IN_PROGRESS]",
        Status::Closed => "[CLOSED]",
    };

    if visited.contains(&issue.id) {
        if is_root {
            println!("{}: {} [CYCLE]", issue.id, truncate(&issue.title, 30));
        } else {
            let connector = if is_last { "└── " } else { "├── " };
            println!("{}{}{}: {} [CYCLE]", prefix, connector, issue.id, truncate(&issue.title, 30));
        }
        return;
    }
    visited.push(issue.id.clone());

    if is_root {
        println!("{}: {} {}", issue.id, truncate(&issue.title, 30), status_tag);
    } else {
        let connector = if is_last { "└── " } else { "├── " };
        println!("{}{}{}: {} {}", prefix, connector, issue.id, truncate(&issue.title, 30), status_tag);
    }

    let new_prefix = if is_root {
        "".to_string()
    } else if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    };

    let blockers = &issue.blocked_by;
    for (i, blocker_id) in blockers.iter().enumerate() {
        let is_last_child = i == blockers.len() - 1;
        if let Some(blocker) = store.issues.get(blocker_id) {
            print_tree_node(store, blocker, &new_prefix, false, is_last_child, visited);
        } else {
            let child_connector = if is_last_child { "└── " } else { "├── " };
            println!("{}{}{} [MISSING]", new_prefix, child_connector, blocker_id);
        }
    }

    visited.pop();
}

fn cmd_cycles(store: &Store, json_output: bool) -> Result<(), String> {
    let mut cycles: Vec<Vec<String>> = vec![];

    for id in store.issues.keys() {
        let mut visited = vec![];
        let mut path = vec![];
        find_cycles(store, id, &mut visited, &mut path, &mut cycles);
    }

    // Deduplicate cycles (same cycle can be found from different starting points)
    let mut unbaue_cycles: Vec<Vec<String>> = vec![];
    for cycle in cycles {
        let normalized = normalize_cycle(&cycle);
        if !unbaue_cycles.iter().any(|c| normalize_cycle(c) == normalized) {
            unbaue_cycles.push(cycle);
        }
    }

    if json_output {
        println!("{}", serde_json::to_string(&unbaue_cycles).unwrap());
        return Ok(());
    }

    if unbaue_cycles.is_empty() {
        println!("No cycles detected.");
    } else {
        println!("Found {} cycle(s):", unbaue_cycles.len());
        for (i, cycle) in unbaue_cycles.iter().enumerate() {
            println!("  {}. {} -> {}", i + 1, cycle.join(" -> "), cycle[0]);
        }
    }

    Ok(())
}

fn find_cycles(
    store: &Store,
    id: &str,
    visited: &mut Vec<String>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    if path.contains(&id.to_string()) {
        // Found a cycle
        let cycle_start = path.iter().position(|x| x == id).unwrap();
        let cycle: Vec<String> = path[cycle_start..].to_vec();
        cycles.push(cycle);
        return;
    }

    if visited.contains(&id.to_string()) {
        return;
    }

    visited.push(id.to_string());
    path.push(id.to_string());

    if let Some(issue) = store.issues.get(id) {
        for blocker in &issue.blocked_by {
            find_cycles(store, blocker, visited, path, cycles);
        }
    }

    path.pop();
}

fn normalize_cycle(cycle: &[String]) -> Vec<String> {
    if cycle.is_empty() {
        return vec![];
    }
    // Rotate so smallest element is first
    let min_pos = cycle
        .iter()
        .enumerate()
        .min_by_key(|(_, v)| *v)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let mut normalized: Vec<String> = cycle[min_pos..].to_vec();
    normalized.extend(cycle[..min_pos].to_vec());
    normalized
}

fn cmd_claim(store: &mut Store, id: &str, session: &str, json_output: bool) -> Result<(), String> {
    let issue = store.issues.get_mut(id).ok_or_else(|| format!("Issue not found: {}", id))?;

    issue.apply(Transition::Claim { session: session.to_string() })?;

    let issue_clone = issue.clone();
    store.save()?;

    if json_output {
        println!("{}", serde_json::to_string(&issue_clone).unwrap());
    } else {
        println!("Claimed {} for session {}", id, session);
    }

    Ok(())
}

fn cmd_release(store: &mut Store, id: &str, json_output: bool) -> Result<(), String> {
    let issue = store.issues.get_mut(id).ok_or_else(|| format!("Issue not found: {}", id))?;

    let old_session = issue.apply(Transition::Release)?;

    let issue_clone = issue.clone();
    store.save()?;

    if json_output {
        println!("{}", serde_json::to_string(&issue_clone).unwrap());
    } else {
        println!("Released {} (was claimed by {})", id, old_session.unwrap());
    }

    Ok(())
}

fn cmd_finish(store: &mut Store, id: &str, json_output: bool) -> Result<(), String> {
    let issue = store.issues.get_mut(id).ok_or_else(|| format!("Issue not found: {}", id))?;

    let old_session = issue.apply(Transition::Finish)?;

    let issue_clone = issue.clone();
    store.save()?;

    if json_output {
        println!("{}", serde_json::to_string(&issue_clone).unwrap());
    } else {
        println!("Finished {} (was claimed by {})", id, old_session.unwrap());
    }

    Ok(())
}

fn cmd_mine(store: &Store, session: &str, json_output: bool) -> Result<(), String> {
    let mut mine: Vec<_> = store
        .issues
        .values()
        .filter(|i| i.session_id.as_deref() == Some(session))
        .collect();

    mine.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.created_at.cmp(&b.created_at))
    });

    if json_output {
        println!("{}", serde_json::to_string(&mine).unwrap());
        return Ok(());
    }

    if mine.is_empty() {
        println!("No issues claimed by session {}", session);
        return Ok(());
    }

    println!();
    println!(
        "  {:<8} {:>2}  {:<8} {}",
        "ID", "P", "TYPE", "TITLE"
    );
    println!("  {}", "-".repeat(60));

    for issue in &mine {
        println!(
            "  {:<8} {:>2}  {:<8} {}",
            issue.id,
            issue.priority,
            issue.issue_type,
            truncate(&issue.title, 40)
        );
    }

    println!();
    println!("{} issue(s) claimed by session {}", mine.len(), session);

    Ok(())
}

fn cmd_label(store: &mut Store, id: &str, action: &str, label: &str, json_output: bool) -> Result<(), String> {
    let issue = store.issues.get_mut(id).ok_or_else(|| format!("Issue not found: {}", id))?;

    match action {
        "add" => {
            if issue.labels.contains(&label.to_string()) {
                return Err(format!("Label '{}' already exists on {}", label, id));
            }
            issue.labels.push(label.to_string());
            issue.labels.sort();
        }
        "remove" => {
            if !issue.labels.contains(&label.to_string()) {
                return Err(format!("Label '{}' not found on {}", label, id));
            }
            issue.labels.retain(|l| l != label);
        }
        _ => return Err(format!("Unknown action: {} (use 'add' or 'remove')", action)),
    }

    issue.updated_at = Utc::now();
    let issue_clone = issue.clone();
    store.save()?;

    if json_output {
        println!("{}", serde_json::to_string(&issue_clone).unwrap());
    } else {
        println!("{} label '{}' {} {}",
            if action == "add" { "Added" } else { "Removed" },
            label,
            if action == "add" { "to" } else { "from" },
            id
        );
    }

    Ok(())
}

fn cmd_priority(store: &mut Store, id: &str, value: u8, json_output: bool) -> Result<(), String> {
    if value > 4 {
        return Err("Priority must be 0-4".to_string());
    }

    let issue = store.issues.get_mut(id).ok_or_else(|| format!("Issue not found: {}", id))?;

    let old_priority = issue.priority;
    issue.priority = value;
    issue.updated_at = Utc::now();

    let issue_clone = issue.clone();
    store.save()?;

    if json_output {
        println!("{}", serde_json::to_string(&issue_clone).unwrap());
    } else {
        println!("Priority {} -> {} for {}", old_priority, value, id);
    }

    Ok(())
}

fn cmd_comment(store: &mut Store, id: &str, text: &str, author: &str, json_output: bool) -> Result<(), String> {
    let issue = store.issues.get_mut(id).ok_or_else(|| format!("Issue not found: {}", id))?;

    let comment = Comment {
        author: author.to_string(),
        text: text.to_string(),
        created_at: Utc::now(),
    };

    issue.comments.push(comment.clone());
    issue.updated_at = Utc::now();

    let issue_clone = issue.clone();
    store.save()?;

    if json_output {
        println!("{}", serde_json::to_string(&comment).unwrap());
    } else {
        println!("Added comment to {} ({} comments total)", id, issue_clone.comments.len());
    }

    Ok(())
}

fn cmd_import(store: &mut Store, file: &Path, keep_ids: bool, json_output: bool) -> Result<(), String> {
    use std::io::BufRead;

    let file_handle = File::open(file)
        .map_err(|e| format!("Failed to open '{}': {}", file.display(), e))?;
    let reader = BufReader::new(file_handle);

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors: Vec<ImportError> = vec![];
    let mut id_map: HashMap<String, String> = HashMap::new(); // old_id -> new_id

    // First pass: parse all issues and build ID map
    let mut beads_issues: Vec<(usize, BeadsIssue)> = vec![];

    for (line_num, line) in reader.lines().enumerate() {
        let line_num = line_num + 1; // 1-indexed for user display
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                errors.push(ImportError {
                    line_num,
                    issue_id: None,
                    field: "line".to_string(),
                    message: format!("Failed to read: {}", e),
                });
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        // First try to get the ID for better error messages
        let raw: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                errors.push(ImportError {
                    line_num,
                    issue_id: None,
                    field: "json".to_string(),
                    message: format!("Invalid JSON: {}", e),
                });
                continue;
            }
        };

        let issue_id = raw.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());

        // Now parse as BeadsIssue
        let beads_issue: BeadsIssue = match serde_json::from_value(raw.clone()) {
            Ok(i) => i,
            Err(e) => {
                // Try to identify which field failed
                let field = if raw.get("title").is_none() {
                    "title (missing)"
                } else if raw.get("status").is_none() {
                    "status (missing)"
                } else if raw.get("issue_type").is_none() {
                    "issue_type (missing)"
                } else if raw.get("created_at").is_none() {
                    "created_at (missing)"
                } else if raw.get("updated_at").is_none() {
                    "updated_at (missing)"
                } else {
                    "parsing"
                };
                errors.push(ImportError {
                    line_num,
                    issue_id,
                    field: field.to_string(),
                    message: format!("{}", e),
                });
                continue;
            }
        };

        beads_issues.push((line_num, beads_issue));
    }

    // Build ID map (before creating issues, so we can resolve dependencies)
    for (_, beads) in &beads_issues {
        let new_id = if keep_ids {
            beads.id.clone()
        } else {
            // Parse timestamp for ID generation
            let ts = DateTime::parse_from_rfc3339(&beads.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            store.generate_id(&beads.title, &ts)
        };
        id_map.insert(beads.id.clone(), new_id);
    }

    // Second pass: create issues with resolved dependencies
    for (line_num, beads) in beads_issues {
        let new_id = id_map.get(&beads.id).unwrap().clone();

        // Check for duplicate
        if store.issues.contains_key(&new_id) {
            skipped += 1;
            continue;
        }

        // Parse status
        let status = match beads.status.as_str() {
            "open" => Status::Open,
            "in_progress" => Status::InProgress,
            "closed" => Status::Closed,
            other => {
                errors.push(ImportError {
                    line_num,
                    issue_id: Some(beads.id.clone()),
                    field: "status".to_string(),
                    message: format!("Unknown status '{}', expected open/in_progress/closed", other),
                });
                continue;
            }
        };

        // Parse issue_type (unknown types map to task)
        let issue_type: IssueType = match beads.issue_type.parse() {
            Ok(t) => t,
            Err(_) => {
                errors.push(ImportError {
                    line_num,
                    issue_id: Some(beads.id.clone()),
                    field: "issue_type".to_string(),
                    message: format!("Unknown type '{}' mapped to 'task'", beads.issue_type),
                });
                IssueType::Task
            }
        };

        // Parse timestamps
        let created_at = match DateTime::parse_from_rfc3339(&beads.created_at) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(e) => {
                errors.push(ImportError {
                    line_num,
                    issue_id: Some(beads.id.clone()),
                    field: "created_at".to_string(),
                    message: format!("Invalid timestamp '{}': {}", beads.created_at, e),
                });
                continue;
            }
        };

        let updated_at = match DateTime::parse_from_rfc3339(&beads.updated_at) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(e) => {
                errors.push(ImportError {
                    line_num,
                    issue_id: Some(beads.id.clone()),
                    field: "updated_at".to_string(),
                    message: format!("Invalid timestamp '{}': {}", beads.updated_at, e),
                });
                continue;
            }
        };

        let closed_at = if let Some(ref ca) = beads.closed_at {
            match DateTime::parse_from_rfc3339(ca) {
                Ok(dt) => Some(dt.with_timezone(&Utc)),
                Err(e) => {
                    errors.push(ImportError {
                        line_num,
                        issue_id: Some(beads.id.clone()),
                        field: "closed_at".to_string(),
                        message: format!("Invalid timestamp '{}': {}", ca, e),
                    });
                    continue;
                }
            }
        } else {
            None
        };

        // Build blocked_by from dependencies where this issue depends on another
        let mut blocked_by: Vec<String> = vec![];
        for dep in &beads.dependencies {
            if dep.dep_type == "blocks" && dep.issue_id == beads.id {
                if let Some(new_blocker_id) = id_map.get(&dep.depends_on_id) {
                    blocked_by.push(new_blocker_id.clone());
                }
            }
        }

        let issue = Issue {
            id: new_id.clone(),
            title: beads.title,
            description: beads.description,
            status,
            priority: beads.priority.min(4),
            issue_type,
            session_id: None,
            labels: vec![],
            comments: vec![],
            created_at,
            updated_at,
            closed_at,
            blocks: vec![], // Will be filled in next pass
            blocked_by,
        };

        store.issues.insert(new_id, issue);
        imported += 1;
    }

    // Third pass: populate `blocks` field (reverse of blocked_by)
    let ids: Vec<String> = store.issues.keys().cloned().collect();
    for id in ids {
        let blocked_by = store.issues.get(&id).unwrap().blocked_by.clone();
        for blocker_id in blocked_by {
            if let Some(blocker) = store.issues.get_mut(&blocker_id) {
                if !blocker.blocks.contains(&id) {
                    blocker.blocks.push(id.clone());
                }
            }
        }
    }

    store.save()?;

    if json_output {
        println!(r#"{{"imported":{},"skipped":{},"errors":{}}}"#,
            imported, skipped, errors.len());
    } else {
        println!("Imported {} issues ({} skipped, {} errors)", imported, skipped, errors.len());
        if !errors.is_empty() {
            println!();
            println!("Errors:");
            for err in &errors {
                println!("  {}", err);
            }
        }
    }

    Ok(())
}

fn cmd_quickstart() {
    println!(r#"
ba - Simple Task Tracking for LLM Sessions

GETTING STARTED
  ba init           Initialize ba in your project (creates .ba/)
  ba quickstart     Show this guide

CREATING ISSUES
  ba create "Fix login bug" -p 1
  ba create "Add caching layer" -t refactor -d "Description here"
  ba create "Research auth options" -t spike -p 2

ISSUE TYPES: task (default), epic, refactor, spike
PRIORITIES: 0 (critical) → 4 (backlog), default is 2

VIEWING ISSUES
  ba list           List open/in_progress issues
  ba list --all     Include closed
  ba list --status open
  ba show <id>      Show full details
  ba ready          Show issues ready to work on (open + not blocked)

OWNERSHIP-BASED WORKFLOW
  ba claim <id> --session $SESSION    Take ownership (open → in_progress)
  ba release <id>                     Abandon work (in_progress → open)
  ba finish <id>                      Complete work (in_progress → closed)
  ba close <id>                       Close unclaimed issue (escape hatch)

  Status is a side-effect of ownership transitions, not set directly.

MODIFYING ISSUES
  ba priority <id> <0-4>              Set priority (0 = critical)
  ba label <id> add urgent            Add a label
  ba label <id> remove urgent         Remove a label
  ba comment <id> "text" --author X   Add a comment

DEPENDENCIES
  ba block <id> <blocker>    Mark <id> blocked by <blocker>
  ba unblock <id> <blocker>  Remove block
  ba tree <id>               Show dependency tree
  ba cycles                  Detect circular dependencies

MULTI-AGENT COORDINATION
  ba claim <id> --session <session_id>  Claim issue for your session
  ba mine --session <session_id>        Show your claimed issues
  ba release <id>                       Release claim (back to pool)

  Tip: Use your Claude session ID as --session value

IMPORTING FROM BEADS (bd)
  ba import .beads/issues.jsonl --keep-ids

JSON OUTPUT (for programmatic use)
  ba --json list
  ba --json show <id>
  ba --json ready

TYPICAL WORKFLOW
  1. ba ready                          # Find unblocked work
  2. ba claim <id> --session $SESSION  # Claim it (sets in_progress)
  3. ... do the work ...
  4. ba finish <id>                    # Complete (clears claim, closes)

DISCOVERING NEW WORK
  1. ba create "Found bug in X" -t bug -p 1
  2. ba block <current_id> <new_id>    # If it blocks current work
  3. ba tree <current_id>              # Verify dependency chain
"#);
}

fn cmd_ready(store: &Store, json_output: bool) -> Result<(), String> {
    // Ready = open issues where all blockers are closed (or no blockers)
    let mut ready: Vec<_> = store
        .issues
        .values()
        .filter(|issue| {
            // Must be open
            if issue.status != Status::Open {
                return false;
            }
            // All blockers must be closed
            issue.blocked_by.iter().all(|blocker_id| {
                store
                    .issues
                    .get(blocker_id)
                    .map(|b| b.status == Status::Closed)
                    .unwrap_or(true) // Missing blocker = not blocking
            })
        })
        .collect();

    // Sort by priority, then by created_at
    ready.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.created_at.cmp(&b.created_at))
    });

    if json_output {
        println!("{}", serde_json::to_string(&ready).unwrap());
        return Ok(());
    }

    if ready.is_empty() {
        println!("No issues ready to work on.");
        return Ok(());
    }

    println!();
    println!(
        "  {:<8} {:>2}  {:<8} {}",
        "ID", "P", "TYPE", "TITLE"
    );
    println!("  {}", "-".repeat(60));

    for issue in &ready {
        println!(
            "  {:<8} {:>2}  {:<8} {}",
            issue.id,
            issue.priority,
            issue.issue_type,
            truncate(&issue.title, 40)
        );
    }

    println!();
    println!("{} issue(s) ready", ready.len());

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => cmd_init(&cli.dir),
        Commands::Quickstart => {
            cmd_quickstart();
            Ok(())
        }
        _ => {
            // All other commands need a loaded store
            match Store::load(&cli.dir) {
                Ok(mut store) => match cli.command {
                    Commands::Init | Commands::Quickstart => unreachable!(),
                    Commands::Create {
                        title,
                        issue_type,
                        priority,
                        description,
                    } => cmd_create(&mut store, title, issue_type, priority, description, cli.json),
                    Commands::List { status, all } => cmd_list(&store, status, all, cli.json),
                    Commands::Show { id } => cmd_show(&store, &id, cli.json),
                    Commands::Close { id, reason } => cmd_close(&mut store, &id, reason, cli.json),
                    Commands::Block { id, blocker } => cmd_block(&mut store, &id, &blocker, cli.json),
                    Commands::Unblock { id, blocker } => cmd_unblock(&mut store, &id, &blocker, cli.json),
                    Commands::Tree { id } => cmd_tree(&store, &id, cli.json),
                    Commands::Cycles => cmd_cycles(&store, cli.json),
                    Commands::Ready => cmd_ready(&store, cli.json),
                    Commands::Claim { id, session } => cmd_claim(&mut store, &id, &session, cli.json),
                    Commands::Release { id } => cmd_release(&mut store, &id, cli.json),
                    Commands::Finish { id } => cmd_finish(&mut store, &id, cli.json),
                    Commands::Mine { session } => cmd_mine(&store, &session, cli.json),
                    Commands::Label { id, action, label } => cmd_label(&mut store, &id, &action, &label, cli.json),
                    Commands::Priority { id, value } => cmd_priority(&mut store, &id, value, cli.json),
                    Commands::Comment { id, text, author } => cmd_comment(&mut store, &id, &text, &author, cli.json),
                    Commands::Import { file, keep_ids } => cmd_import(&mut store, &file, keep_ids, cli.json),
                },
                Err(e) => Err(e),
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
