# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`ba` is a simple task tracking CLI for LLM sessions, written in Rust. It's a spiritual fork of [beads](https://github.com/steveyegge/beads), keeping the simplicity of v0.9.6 with added ownership-based state machine for multi-agent coordination.

## Build & Development

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Run
cargo run -- <command>   # Run with arguments
cargo run -- quickstart  # Show help

# Install locally
cargo install --path .

# Test (no separate test suite - single file crate)
cargo check              # Type check
cargo clippy             # Lint
```

## Architecture

### Single-file Crate

The entire implementation is in `src/main.rs` (~1700 lines). Key sections:

1. **Data Types** (lines 18-130): `Status`, `IssueType`, `Transition`, `Comment`, `Issue`, `Config`
2. **State Machine** (lines 82-256): `Issue::apply()` implements ownership-based transitions
3. **Store** (lines 314-421): In-memory HashMap + JSONL file operations
4. **CLI** (lines 423-590): Clap-based command definitions
5. **Command Implementations** (lines 592-1629): One function per command

### Storage Model

```
.ba/
├── config.json       # Project config (version, ID prefix)
└── issues.jsonl      # One issue per line, sorted by ID
```

- **JSONL**: Git-friendly (one issue per line = per-issue conflicts)
- **No database**: Just files, no SQLite or daemon
- **Atomic writes**: Uses temp file + rename

### ID Generation

Format: `{prefix}-{random}` (e.g., `ab-x7k2`)
- `prefix`: 2 chars derived from project path hash (same project = same prefix)
- `random`: 4 chars from SHA256 of title + timestamp, with sliding window for collision avoidance

### Ownership-Based State Machine

Status is a side-effect of ownership transitions, not set directly:

```
                 claim                    release
    ┌─────────────────────────────────────────────────────┐
    │                                                     │
    ▼                                                     │
  OPEN ──────claim──────► IN_PROGRESS ─────finish─────► CLOSED
    │                          ▲                          │
    │                          │                          │
    └──────────close───────────┼─────────claim────────────┘
           (escape hatch)      │        (reopen)
```

Key transitions in `Issue::apply()`:
- `Claim`: (Open|Closed) → InProgress (assigns session_id)
- `Release`: InProgress → Open (clears session_id)
- `Finish`: InProgress → Closed (clears session_id, sets closed_at)
- `Close`: Open → Closed (escape hatch for unclaimed issues)

### Dependencies

Bidirectional blocking relationships:
- `issue.blocked_by`: IDs that block this issue
- `issue.blocks`: IDs this issue blocks
- An issue is "ready" when: status == Open AND all blockers are Closed

### Issue Types

Minimal by design - only types that signal different work patterns:
- `task` (default): General work
- `epic`: Container for grouping
- `refactor`: Improving existing code
- `spike`: Research/investigation

Legacy types (`bug`, `feature`, `chore`) deserialize to `Task` for backwards compatibility via `#[serde(other)]`.

## Key Implementation Details

### Beads Import

`cmd_import()` handles migration from beads (bd) exports:
1. First pass: Parse issues, build old_id → new_id map
2. Second pass: Create issues with resolved dependency IDs
3. Third pass: Populate reverse `blocks` relationships

Only `blocks` dependency type is imported (others like `related`, `parent-child` are skipped).

### JSON Output

All commands support `--json` flag for programmatic use. The flag is parsed at CLI level and passed to each command function.

## Workflow Reference

```bash
ba ready                              # Find unblocked work
ba claim <id> --session $SESSION      # Take ownership (→ in_progress)
... do the work ...
ba finish <id>                        # Complete (→ closed)
```

See `ba quickstart` for full command reference.
