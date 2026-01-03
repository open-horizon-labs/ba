# ac

Simple task tracking for LLM sessions.

```
ac - because sometimes you need to go back before bd
```

A spiritual fork of [beads](https://github.com/steveyegge/beads) (`bd`), keeping the simplicity of v0.9.6 with added session-based claiming for multi-agent coordination.

## Philosophy

- **Dead simple**: Single binary, minimal dependencies
- **Plain text**: JSONL files, human-readable, git-friendly
- **Multi-agent**: Sessions claim issues, obvious ownership
- **Zero infrastructure**: No SQLite, no daemon - just files

## Installation

```bash
cargo install ac
# or build from source
cargo build --release
```

## Quick Start

```bash
# Initialize in your project
ac init

# Create issues
ac create "Fix auth bug" -t bug -p 1
ac create "Add feature" -t feature -d "Description here"

# List issues (excludes closed by default)
ac list
ac list --all              # Include closed
ac list --status open      # Filter by status

# Show issue details
ac show ab-x7k2

# Update issues
ac update ab-x7k2 --status in_progress
ac update ab-x7k2 --priority 0
ac update ab-x7k2 --assignee alice

# Close issues
ac close ab-x7k2 --reason "Fixed in commit abc123"
```

## Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item
- `epic` - Large feature composed of subtasks
- `chore` - Maintenance work
- `refactor` - Code restructuring
- `spike` - Research/investigation

## Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default - nice-to-have features, minor bugs)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

## Storage

Data stored in `.ac/` directory:
- `config.json` - Project config (version, ID prefix)
- `issues.jsonl` - One issue per line, sorted by ID

### Why JSONL?

- **Git-friendly**: One issue per line = conflicts are per-issue
- **Human-readable**: Easy to inspect with standard tools
- **Grep-able**: `grep ab-x7k2 .ac/issues.jsonl`

## IDs

Format: `{prefix}-{random}` (e.g., `ab-x7k2`)

- **prefix**: 2 chars derived from project path hash
- **random**: 4 chars lowercase alphanumeric

Same project always gets same prefix, different projects get different prefixes.

## JSON Output

All commands support `--json` for programmatic use:

```bash
ac --json list
ac --json show ab-x7k2
ac --json create "New issue" -t task
```

## Acknowledgment

`ac` is inspired by [beads](https://github.com/steveyegge/beads) by Steve Yegge - an excellent issue tracker for AI-assisted development. We loved beads v0.9.6's simplicity before it evolved into a full messaging/routing system. `ac` takes that original simplicity and adds session-based claiming for multi-agent coordination.

## License

MIT
