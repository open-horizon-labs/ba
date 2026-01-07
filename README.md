# ba

Simple task tracking for LLM sessions.

```
ba - because sometimes you need to go back before bd
```

A spiritual fork of [beads](https://github.com/steveyegge/beads) (`bd`), keeping the simplicity of v0.9.6 with added session-based claiming for multi-agent coordination.

## Philosophy

- **Dead simple**: Single binary, minimal dependencies
- **Plain text**: JSONL files, human-readable, git-friendly
- **Multi-agent**: Sessions claim issues, obvious ownership
- **Zero infrastructure**: No SQLite, no daemon - just files

## Installation

### Binary

```bash
cargo install ba
# or build from source
cargo build --release
```

### Claude Code Plugin

```bash
# Clone this repository
git clone https://github.com/cloud-atlas-ai/ba.git
cd ba

# Add ba marketplace from local directory
claude plugin marketplace add $PWD

# Install ba plugin (includes Codex skill)
claude plugin install ba@ba
```

After [PR #1](https://github.com/cloud-atlas-ai/ba/pull/1) merges to master:
```bash
# Simpler: install directly from GitHub
claude plugin marketplace add https://github.com/cloud-atlas-ai/ba
claude plugin install ba@ba
```

The plugin provides:
- `/ba init` - Install ba binary, initialize project, install Codex skill, setup AGENTS.md
- `/ba status` - Show issue counts and your claimed work
- `/ba quickstart` - Quick reference guide

After running `/ba init`, the `$ba` Codex skill will be available for task tracking commands.

See [plugin/README.md](plugin/README.md) and [codex-skill/README.md](codex-skill/README.md) for details.

## Quick Start

```bash
# Show quick start guide for LLMs
ba quickstart

# Initialize in your project
ba init

# Create issues
ba create "Fix auth bug" -t bug -p 1
ba create "Add feature" -t feature -d "Description here"

# List issues (excludes closed by default)
ba list
ba list --all              # Include closed
ba list --status open      # Filter by status

# Show issue details
ba show ab-x7k2
```

## Ownership-Based Workflow

Status is a side-effect of ownership transitions, not set directly:

```bash
# Take ownership (open/closed → in_progress)
ba claim ab-x7k2 --session claude-abc123

# Abandon work (in_progress → open)
ba release ab-x7k2

# Complete work (in_progress → closed)
ba finish ab-x7k2

# Close unclaimed issue (escape hatch)
ba close ab-x7k2
```

This ensures every in-progress issue has an owner. Claiming a closed issue cleanly reopens it.

## Modifying Issues

```bash
# Change priority
ba priority ab-x7k2 0      # 0 = critical

# Add/remove labels
ba label ab-x7k2 add urgent
ba label ab-x7k2 remove urgent

# Add comments
ba comment ab-x7k2 "Found root cause" --author claude
```

## Dependencies

Track blocking relationships between issues:

```bash
# Add a blocking dependency (blocker blocks id)
ba block ab-x7k2 ab-y8m3    # ab-x7k2 is now blocked by ab-y8m3

# Remove a blocking dependency
ba unblock ab-x7k2 ab-y8m3

# Visualize dependency tree
ba tree ab-x7k2
# Output:
# ab-x7k2: Fix auth bug [OPEN]
# └── ab-y8m3: Add user model [IN_PROGRESS]

# Detect circular dependencies
ba cycles
```

## Ready Queue

Show issues ready to work on (open + not blocked):

```bash
ba ready
# Output:
#   ID        P  TYPE     TITLE
#   ------------------------------------------------------------
#   ab-x7k2   0  bug      Fix critical auth bug
#   ab-z9n4   1  feature  Add dashboard
#   ab-a1b2   2  task     Write tests
#
# 3 issue(s) ready
```

An issue is "ready" when:
- Status is `open` (not `in_progress` or `closed`)
- All blocking issues are `closed` (or has no blockers)

## Multi-Agent Coordination

When multiple LLM agents work on the same codebase:

```bash
# Claim an issue (caller provides their session ID)
ba claim ab-x7k2 --session claude-abc123

# See what you've claimed
ba mine --session claude-abc123

# Complete work
ba finish ab-x7k2

# Or release back to pool
ba release ab-x7k2
```

The ownership model ensures no two agents work on the same issue. See [Ownership-Based Workflow](#ownership-based-workflow) above.

## Importing from Beads

Migrate issues from a beads (`bd`) export file:

```bash
# Import with new IDs (uses project prefix)
ba import .beads/issues.jsonl

# Keep original beads IDs
ba import .beads/issues.jsonl --keep-ids
```

The import handles dependencies automatically and provides clear error messages:

```
Imported 112 issues (0 skipped, 1 errors)

Errors:
  Line 46: Issue 'as-9q7' - issue_type: Unknown type 'merge-request', expected bug/feature/task/epic/chore/refactor/spike
```

Only `blocks` dependencies are imported (other types like `related`, `parent-child`, `discovered-from` are skipped).

## Issue Types

- `task` - Default, general work item
- `epic` - Container for grouping related issues
- `refactor` - Improving existing code (no new behavior)
- `spike` - Research/investigation (may not produce code)

## Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default - nice-to-have features, minor bugs)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

## Storage

Data stored in `.ba/` directory:
- `config.json` - Project config (version, ID prefix)
- `issues.jsonl` - One issue per line, sorted by ID

### Why JSONL?

- **Git-friendly**: One issue per line = conflicts are per-issue
- **Human-readable**: Easy to inspect with standard tools
- **Grep-able**: `grep ab-x7k2 .ba/issues.jsonl`

## IDs

Format: `{prefix}-{random}` (e.g., `ab-x7k2`)

- **prefix**: 2 chars derived from project path hash
- **random**: 4 chars lowercase alphanumeric

Same project always gets same prefix, different projects get different prefixes.

## JSON Output

All commands support `--json` for programmatic use:

```bash
ba --json list
ba --json show ab-x7k2
ba --json create "New issue" -t task
```

## Acknowledgment

`ba` is inspired by [beads](https://github.com/steveyegge/beads) by Steve Yegge - an excellent issue tracker for AI-assisted development. We loved beads v0.9.6's simplicity before it evolved into a full messaging/routing system. `ba` takes that original simplicity and adds an ownership-based state machine for multi-agent coordination.

## License

Source-available. See [LICENSE](LICENSE) for details.
