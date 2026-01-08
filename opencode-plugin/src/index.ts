/**
 * ba OpenCode Plugin
 *
 * Simple task tracking for LLM sessions - ownership-based workflow for multi-agent coordination
 */

import type { Plugin } from "@opencode-ai/plugin";
import { tool } from "@opencode-ai/plugin";
import { existsSync, readFileSync, mkdirSync, writeFileSync, appendFileSync } from "fs";
import { join } from "path";
import { execSync } from "child_process";

// Check if ba binary is available
function isBaAvailable(): boolean {
  try {
    execSync("command -v ba", { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

// Check if package manager is available
function checkPackageManager(manager: string): boolean {
  try {
    execSync(`command -v ${manager}`, { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

// Execute ba command and return output
function execBa(args: string): string {
  try {
    return execSync(`ba ${args}`, { encoding: "utf-8", maxBuffer: 10 * 1024 * 1024 });
  } catch (error: any) {
    throw new Error(`ba command failed: ${error.message}`);
  }
}

// Check if .ba directory exists
function isBaInitialized(directory: string): boolean {
  return existsSync(join(directory, ".ba"));
}

export const BA: Plugin = async ({ directory }) => {
  return {
    tool: {
      "ba-init": tool({
        description: "Initialize ba task tracking for this project. Checks for ba binary, guides installation if needed, and sets up the project.",
        args: {},
        async execute() {
          // Step 1: Check if already initialized
          if (isBaInitialized(directory)) {
            const statusOutput = execBa("list --json");
            const issues = JSON.parse(statusOutput);
            return `ba is already initialized.\n\n${issues.length} issues in project. Use 'ba-status' to see details.`;
          }

          // Step 2: Check if ba binary is available
          if (!isBaAvailable()) {
            const hasBrew = checkPackageManager("brew");
            const hasCargo = checkPackageManager("cargo");

            let installGuide = "ba binary not found. Installation options:\n\n";

            if (hasBrew) {
              installGuide += "Via Homebrew (recommended for macOS):\n";
              installGuide += "  brew install cloud-atlas-ai/ba/ba\n\n";
            }

            if (hasCargo) {
              installGuide += "Via Cargo:\n";
              installGuide += "  cargo install ba\n\n";
            }

            if (!hasBrew && !hasCargo) {
              installGuide += "No package manager found. Install one first:\n\n";
              installGuide += "Homebrew (recommended for macOS):\n";
              installGuide += "  /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"\n";
              installGuide += "  Then: brew install cloud-atlas-ai/ba/ba\n\n";
              installGuide += "Or Rust/Cargo (cross-platform):\n";
              installGuide += "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\n";
              installGuide += "  Then restart shell and: cargo install ba\n";
            }

            installGuide += "\nAfter installing ba, run this command again to initialize the project.";
            return installGuide;
          }

          // Step 3: Initialize project
          try {
            const output = execBa("init");

            // Step 4: Update AGENTS.md
            const agentsPath = join(directory, "AGENTS.md");
            const baSection = `
## ba Task Tracking

This project uses ba for task tracking.

**Core workflow:**
1. Check ready queue: \`ba ready\` - shows open, unblocked issues
2. Claim an issue: \`ba claim <id> --session $SESSION_ID\`
3. Work on the issue
4. Complete: \`ba finish <id>\`
5. Or release: \`ba release <id>\` if not done

**Quick reference:**
- \`ba list\` - Show all open issues (excludes closed)
- \`ba list --all\` - Include closed issues
- \`ba show <id>\` - Show issue details
- \`ba mine --session $SESSION_ID\` - Show your claimed issues
- \`ba create "title" -t task -p 1\` - Create new issue
- \`ba comment <id> "message"\` - Add comment

**Ownership-based state machine:**
- \`open\` → claim → \`in_progress\` (you own it)
- \`in_progress\` → finish → \`closed\`
- \`in_progress\` → release → \`open\` (back to pool)
- Claiming a closed issue reopens it automatically

**Issue types:** task, epic, refactor, spike
**Priorities:** 0 (critical) to 4 (backlog), default 2

See README.md for full details.
`;

            if (existsSync(agentsPath)) {
              appendFileSync(agentsPath, baSection);
            } else {
              const fullAgents = `# AGENTS.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

[Add project-specific overview here]
${baSection}`;
              writeFileSync(agentsPath, fullAgents);
            }

            return `✓ ba initialized and ready

Created .ba/ directory with project config
${existsSync(agentsPath) ? "Updated" : "Created"} AGENTS.md with ba guidance

Quick start:
  ba create "Your first task" -t task
  ba list
  ba claim <id> --session $SESSION_ID

Use ba-status to see current project status.`;
          } catch (error: any) {
            return `Failed to initialize ba: ${error.message}`;
          }
        },
      }),

      "ba-status": tool({
        description: "Show current ba status for this project, including issue counts and your claimed issues.",
        args: {},
        async execute() {
          if (!isBaInitialized(directory)) {
            return "ba not initialized. Use 'ba-init' to set up ba for this project.";
          }

          if (!isBaAvailable()) {
            return "ba binary not found. Use 'ba-init' for installation instructions.";
          }

          try {
            // Get configuration
            const configPath = join(directory, ".ba", "config.json");
            const config = JSON.parse(readFileSync(configPath, "utf-8"));

            // Get issue counts
            const allIssues = JSON.parse(execBa("list --all --json"));
            const openIssues = allIssues.filter((i: any) => i.status !== "closed");
            const closedIssues = allIssues.filter((i: any) => i.status === "closed");
            const inProgressIssues = allIssues.filter((i: any) => i.status === "in_progress");

            // Get ready queue
            const readyOutput = execBa("ready");

            let result = `✓ ba initialized

Project: ${config.id_prefix || "unknown"} (prefix)
Version: ${config.version || "unknown"}

Issues:
  Open: ${openIssues.length}
  In Progress: ${inProgressIssues.length}
  Closed: ${closedIssues.length}

Ready to work:
${readyOutput}`;

            // Check for claimed issues if SESSION_ID is available
            const sessionId = process.env.SESSION_ID;
            if (sessionId) {
              try {
                const mineOutput = execBa(`mine --session ${sessionId}`);
                result += `\n\nYour claimed issues:\n${mineOutput}`;
              } catch {
                // No claimed issues or session not found
              }
            }

            return result;
          } catch (error: any) {
            return `Failed to get ba status: ${error.message}`;
          }
        },
      }),

      "ba-quickstart": tool({
        description: "Show a quick reference guide for using ba in this session.",
        args: {},
        async execute() {
          if (!isBaInitialized(directory)) {
            return "ba not initialized yet. Run 'ba-init' to set up.";
          }

          return `ba - Task Tracking Quick Start
==============================

Core Workflow:
  ba ready                            # See available work
  ba claim <id> --session $SESSION_ID # Take ownership
  ba show <id>                        # Check details
  ba finish <id>                      # Complete work
  ba release <id>                     # Abandon work

Browse:
  ba list                             # All open issues
  ba list --all                       # Include closed
  ba mine --session $SESSION_ID       # Your claimed issues

Create:
  ba create "title" -t task -p 1      # New issue
  ba comment <id> "msg" --author $SESSION_ID

Dependencies:
  ba block <id> <blocker-id>          # Mark as blocked
  ba tree <id>                        # Visualize deps

Ownership State Machine:
  open → claim → in_progress → finish → closed
                      ↓
                  release
                      ↓
                    open

Issue Types:
  task      - General work (default)
  epic      - Grouping container
  refactor  - Improve existing code
  spike     - Research/investigation

Priorities:
  0 - Critical    1 - High    2 - Medium (default)
  3 - Low         4 - Backlog

Session ID:
  Use --session $SESSION_ID for claim/mine/comment
  This identifies you in multi-agent workflows

Storage:
  .ba/issues.jsonl  - One issue per line (git-friendly)
  .ba/config.json   - Project configuration

More info: ba --help or README.md`;
        },
      }),
    },
  };
};

export default BA;
