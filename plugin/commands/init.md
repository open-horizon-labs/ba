# Initialize ba

Initialize ba task tracking for this project.

## Step 1: Check current state
- Check if `.ba/` already exists - if so, tell user it's already initialized and show status
- Check if `ba` binary is available (`command -v ba`) - if yes, skip to Step 3

## Step 2: Install ba binary

**Detect available package managers:**
- Homebrew: `command -v brew`
- Cargo: `command -v cargo` OR `test -f ~/.cargo/bin/cargo`

**Offer installation based on what's available:**

If **Homebrew** available (preferred for macOS):
```bash
brew install cloud-atlas-ai/ba/ba
```

If **Cargo** available:
```bash
cargo install ba
# or if cargo not in PATH:
~/.cargo/bin/cargo install ba
```

If **neither available**, offer to install a package manager:
- **Install Homebrew** (recommended for macOS):
  ```bash
  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
  ```
  Then: `brew install cloud-atlas-ai/ba/ba`

- **Install Rust** (cross-platform):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
  Then restart shell and: `cargo install ba`

**For local development** (if user specifies a path):
```bash
cargo install --path /path/to/ba
# or: ~/.cargo/bin/cargo install --path /path/to/ba
```

## Step 3: Initialize project
After `ba` binary is available, run:
```bash
ba init
```

This creates `.ba/` directory with:
- `config.json` - Project config (version, ID prefix)
- `issues.jsonl` - Issue storage

## Step 4: Update CLAUDE.md

Check if `CLAUDE.md` exists in the project root. If not, create it with:

```markdown
# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

[Add project-specific overview here]

## ba Task Tracking

This project uses ba for task tracking. You have the `$ba` skill available in Codex mode.

**Core workflow:**
1. Check ready queue: `ba ready` - shows open, unblocked issues
2. Claim an issue: `ba claim <id> --session $SESSION_ID`
3. Work on the issue
4. Complete: `ba finish <id>`
5. Or release: `ba release <id>` if not done

**Quick reference:**
- `ba list` - Show all open issues (excludes closed)
- `ba list --all` - Include closed issues
- `ba show <id>` - Show issue details
- `ba mine --session $SESSION_ID` - Show your claimed issues
- `ba create "title" -t task -p 1` - Create new issue
- `ba comment <id> "message"` - Add comment

**Ownership-based state machine:**
- `open` → claim → `in_progress` (you own it)
- `in_progress` → finish → `closed`
- `in_progress` → release → `open` (back to pool)
- Claiming a closed issue reopens it automatically

**Issue types:** task, epic, refactor, spike
**Priorities:** 0 (critical) to 4 (backlog), default 2

See README.md for full details.
```

If `CLAUDE.md` already exists, append the ba section:

```bash
# Quoted heredoc prevents $SESSION_ID expansion in CLAUDE.md
cat >> CLAUDE.md << 'EOF'

## ba Task Tracking

This project uses ba for task tracking. You have the `$ba` skill available in Codex mode.

**Core workflow:**
1. Check ready queue: `ba ready` - shows open, unblocked issues
2. Claim an issue: `ba claim <id> --session $SESSION_ID`
3. Work on the issue
4. Complete: `ba finish <id>`
5. Or release: `ba release <id>` if not done

**Quick reference:**
- `ba list` - Show all open issues (excludes closed)
- `ba list --all` - Include closed issues
- `ba show <id>` - Show issue details
- `ba mine --session $SESSION_ID` - Show your claimed issues
- `ba create "title" -t task -p 1` - Create new issue
- `ba comment <id> "message"` - Add comment

**Ownership-based state machine:**
- `open` → claim → `in_progress` (you own it)
- `in_progress` → finish → `closed`
- `in_progress` → release → `open` (back to pool)
- Claiming a closed issue reopens it automatically

**Issue types:** task, epic, refactor, spike
**Priorities:** 0 (critical) to 4 (backlog), default 2

See README.md for full details.
EOF
```

## Step 5: Verify SESSION_ID

Check if SESSION_ID is available for ownership operations:

```bash
if [ -z "$SESSION_ID" ]; then
  echo ""
  echo "⚠️  SESSION_ID not set"
  echo "Claude Code provides this automatically in active sessions."
  echo "To set manually: export SESSION_ID=\$(uuidgen | tr '[:upper:]' '[:lower:]')"
else
  echo ""
  echo "✓ SESSION_ID is set: $SESSION_ID"
fi
```

## Step 6: Confirm

Tell user:
```
✓ ba initialized and ready

Created .ba/ directory with project config
Added ba guidance to CLAUDE.md

Quick start:
  ba create "Your first task" -t task
  ba list
  ba claim <id> --session $SESSION_ID

For Codex integration, use the $ba skill.
```

---
Be concise. Detect what's available, offer appropriate options, guide user through setup.
