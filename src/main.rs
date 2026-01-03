//! ac - Simple task tracking for LLM sessions
//!
//! A spiritual fork of beads (bd), keeping the simplicity of v0.9.6
//! with added session-based claiming for multi-agent coordination.

use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use rand::Rng;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum IssueType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
    Refactor,
    Spike,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueType::Bug => write!(f, "bug"),
            IssueType::Feature => write!(f, "feature"),
            IssueType::Task => write!(f, "task"),
            IssueType::Epic => write!(f, "epic"),
            IssueType::Chore => write!(f, "chore"),
            IssueType::Refactor => write!(f, "refactor"),
            IssueType::Spike => write!(f, "spike"),
        }
    }
}

impl std::str::FromStr for IssueType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bug" => Ok(IssueType::Bug),
            "feature" => Ok(IssueType::Feature),
            "task" => Ok(IssueType::Task),
            "epic" => Ok(IssueType::Epic),
            "chore" => Ok(IssueType::Chore),
            "refactor" => Ok(IssueType::Refactor),
            "spike" => Ok(IssueType::Spike),
            _ => Err(format!("Unknown issue type: {}", s)),
        }
    }
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
    assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    closed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    blocks: Vec<String>,
    #[serde(default)]
    blocked_by: Vec<String>,
}

fn default_priority() -> u8 {
    2
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
    ac_dir: PathBuf,
}

impl Store {
    fn load(ac_dir: &Path) -> Result<Self, String> {
        let config_path = ac_dir.join(CONFIG_FILE);
        let config: Config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse config: {}", e))?
        } else {
            return Err("Not initialized. Run 'ac init' first.".to_string());
        };

        let issues_path = ac_dir.join(ISSUES_FILE);
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
            ac_dir: ac_dir.to_path_buf(),
        })
    }

    fn save(&self) -> Result<(), String> {
        // Sort issues by ID for consistent output
        let mut sorted: Vec<_> = self.issues.values().collect();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));

        let issues_path = self.ac_dir.join(ISSUES_FILE);
        let tmp_path = self.ac_dir.join("issues.jsonl.tmp");

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

    fn generate_id(&self) -> String {
        let mut rng = rand::thread_rng();
        loop {
            let random: String = (0..4)
                .map(|_| {
                    let idx = rng.gen_range(0..36);
                    if idx < 10 {
                        (b'0' + idx) as char
                    } else {
                        (b'a' + idx - 10) as char
                    }
                })
                .collect();
            let id = format!("{}-{}", self.config.prefix, random);
            if !self.issues.contains_key(&id) {
                return id;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CLI
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "ac")]
#[command(about = "Simple task tracking for LLM sessions")]
#[command(version)]
struct Cli {
    /// Data directory (default: .ac/)
    #[arg(long, default_value = ".ac")]
    dir: PathBuf,

    /// Output in JSON format
    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .ac/ directory
    Init,

    /// Create a new issue
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

    /// Update an issue
    Update {
        /// Issue ID
        id: String,

        /// New status (open, in_progress, closed)
        #[arg(long)]
        status: Option<String>,

        /// New priority (0-4)
        #[arg(long)]
        priority: Option<u8>,

        /// New assignee
        #[arg(long)]
        assignee: Option<String>,
    },
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
    let id = store.generate_id();

    let issue = Issue {
        id: id.clone(),
        title,
        description,
        status: Status::Open,
        priority,
        issue_type,
        assignee: None,
        session_id: None,
        labels: vec![],
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
    println!("Type:     {:<16} Assignee: {}", issue.issue_type, issue.assignee.as_deref().unwrap_or("-"));
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

    Ok(())
}

fn cmd_close(store: &mut Store, id: &str, _reason: Option<String>, json_output: bool) -> Result<(), String> {
    let issue = store.issues.get_mut(id).ok_or_else(|| format!("Issue not found: {}", id))?;

    if issue.status == Status::Closed {
        return Err(format!("Issue {} is already closed", id));
    }

    issue.status = Status::Closed;
    issue.closed_at = Some(Utc::now());
    issue.updated_at = Utc::now();

    let issue_clone = issue.clone();
    store.save()?;

    if json_output {
        println!("{}", serde_json::to_string(&issue_clone).unwrap());
    } else {
        println!("Closed {}", id);
    }

    Ok(())
}

fn cmd_update(
    store: &mut Store,
    id: &str,
    status: Option<String>,
    priority: Option<u8>,
    assignee: Option<String>,
    json_output: bool,
) -> Result<(), String> {
    let issue = store.issues.get_mut(id).ok_or_else(|| format!("Issue not found: {}", id))?;

    if let Some(s) = status {
        issue.status = match s.as_str() {
            "open" => Status::Open,
            "in_progress" => Status::InProgress,
            "closed" => {
                issue.closed_at = Some(Utc::now());
                Status::Closed
            }
            _ => return Err(format!("Unknown status: {}", s)),
        };
    }

    if let Some(p) = priority {
        if p > 4 {
            return Err("Priority must be 0-4".to_string());
        }
        issue.priority = p;
    }

    if let Some(a) = assignee {
        issue.assignee = if a.is_empty() { None } else { Some(a) };
    }

    issue.updated_at = Utc::now();

    let issue_clone = issue.clone();
    store.save()?;

    if json_output {
        println!("{}", serde_json::to_string(&issue_clone).unwrap());
    } else {
        println!("Updated {}", id);
    }

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
        _ => {
            // All other commands need a loaded store
            match Store::load(&cli.dir) {
                Ok(mut store) => match cli.command {
                    Commands::Init => unreachable!(),
                    Commands::Create {
                        title,
                        issue_type,
                        priority,
                        description,
                    } => cmd_create(&mut store, title, issue_type, priority, description, cli.json),
                    Commands::List { status, all } => cmd_list(&store, status, all, cli.json),
                    Commands::Show { id } => cmd_show(&store, &id, cli.json),
                    Commands::Close { id, reason } => cmd_close(&mut store, &id, reason, cli.json),
                    Commands::Update {
                        id,
                        status,
                        priority,
                        assignee,
                    } => cmd_update(&mut store, &id, status, priority, assignee, cli.json),
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
