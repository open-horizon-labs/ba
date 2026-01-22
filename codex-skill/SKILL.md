---
name: ba
description: Task tracker for LLM sessions. Use "$ba ready" to see available work, "$ba claim <id>" to take ownership, "$ba finish <id>" when done.
---

# ba - Task Tracking for LLM Sessions

Simple ownership-based task tracking for multi-agent coordination.

## $ba ready

Show issues ready to work on (open + not blocked by dependencies).

**Run:**
```bash
ba ready
```

**Output shows:**
- Issue ID, priority (0-4), type, title
- Only open issues with no unfinished blockers
- Sorted by priority

**Use this to:** Pick your next task from the ready queue.

## $ba claim <id>

Take ownership of an issue (moves it to `in_progress`).

**Run:**
```bash
ba claim <id> --session $SESSION_ID
```

**Example:**
```bash
ba claim ab-x7k2 --session $SESSION_ID
```

**Important:**
- Always use `--session $SESSION_ID` to identify yourself
- Claiming changes status: `open` → `in_progress`
- Claiming a `closed` issue reopens it automatically
- Only one agent can own an issue at a time

**Use this before:** Starting work on any issue.

## $ba mine

Show issues you currently own.

**Run:**
```bash
ba mine --session $SESSION_ID
```

**Output shows:**
- All issues you've claimed in `in_progress` status
- Sorted by priority

**Use this to:** See what you're currently working on.

## $ba finish <id>

Complete an issue (moves it to `closed`).

**Run:**
```bash
ba finish <id>
```

**Example:**
```bash
ba finish ab-x7k2
```

**Requirements:**
- You must be the current owner (claimed it with your session)
- Changes status: `in_progress` → `closed`

**Use this when:** Work is done and tested.

## $ba release <id>

Release an issue back to the ready queue (abandon work).

**Run:**
```bash
ba release <id>
```

**Requirements:**
- You must be the current owner
- Changes status: `in_progress` → `open`

**Use this when:** You can't complete the work or need to switch focus.

## $ba show <id>

Show detailed information about an issue.

**Run:**
```bash
ba show <id>
```

**Output shows:**
- Full details: status, owner, created_at, updated_at
- Description if present
- Labels and priority
- Comments
- Dependencies (blocks, blocked_by)

**Use this to:** Understand issue requirements before claiming.

## $ba list

List all open issues (excludes closed by default).

**Run:**
```bash
ba list              # Open issues only
ba list --all        # Include closed
ba list --status open
ba list --status in_progress
ba list --status closed
```

**Use this to:** Browse available work.

## $ba create <title>

Create a new issue.

**Run:**
```bash
ba create "Fix auth bug" -t task -p 1
ba create "Add feature" -t epic -d "Detailed description"
```

**Options:**
- `-t, --type` - task, epic, refactor, spike (default: task)
- `-p, --priority` - 0 (critical) to 4 (backlog), default: 2
- `-d, --description` - Longer description
- `-l, --labels` - Comma-separated labels

**Use this when:** You discover new work that needs tracking.

## $ba comment <id> <message>

Add a comment to an issue.

**Run:**
```bash
ba comment ab-x7k2 "Found root cause in auth.rs:42" --author $SESSION_ID
```

**Use this to:** Document progress, findings, or blockers.

## $ba block <id> <blocker-id>

Mark an issue as blocked by another.

**Run:**
```bash
ba block ab-x7k2 ab-y8m3    # ab-x7k2 is blocked by ab-y8m3
```

**Effect:**
- `ab-x7k2` won't appear in `ba ready` until `ab-y8m3` is closed
- Use `ba tree <id>` to visualize dependencies

**Use this when:** Work has prerequisites.

## $ba unblock <id> <blocker-id>

Remove a blocking dependency.

**Run:**
```bash
ba unblock ab-x7k2 ab-y8m3
```

## $ba tree <id>

Visualize issue dependencies.

**Run:**
```bash
ba tree ab-x7k2
```

**Output shows:**
- Tree structure of issue and its blockers
- Status of each issue

## Workflow Example

```bash
# 1. See what's available
ba ready

# 2. Pick an issue and claim it
ba show ab-x7k2              # Check details
ba claim ab-x7k2 --session $SESSION_ID

# 3. Work on it
# ... make changes, run tests ...

# 4. Document progress
ba comment ab-x7k2 "Implemented feature, tests passing" --author $SESSION_ID

# 5. Complete or release
ba finish ab-x7k2            # If done
# OR
ba release ab-x7k2           # If can't complete
```

## Ownership State Machine

ba uses ownership-based state transitions:

```text
open ──claim──> in_progress ──finish──> closed
       (take)                  (done)
                    │
                 release
                    │
                    ▼
                  open
                (abandon)
```

Key rule: **Every `in_progress` issue has an owner.**

Claiming a closed issue automatically reopens it as `in_progress` with you as owner.

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

## JSON Output

All commands support `--json` for programmatic use:

```bash
ba --json ready
ba --json show ab-x7k2
ba --json mine --session $SESSION_ID
```

## Storage

Issues stored in `.ba/issues.jsonl`:
- One issue per line (JSONL format)
- Git-friendly (conflicts are per-issue, not per-field)
- Sorted by ID (stable ordering for conflict resolution)
- Human-readable
- Grep-able: `grep ab-x7k2 .ba/issues.jsonl`

### Merge Conflicts

When multiple agents modify issues concurrently, git conflicts occur at the line level (one line = one issue).

**Resolution strategy:**
1. Pull latest changes: `git pull`
2. If conflicts occur, each conflicting issue will show both versions
3. Choose the correct version (usually the one with latest `updated_at`)
4. Validate JSON: `jq empty .ba/issues.jsonl`
5. Commit the resolution

**Prevention:** Always pull before starting work. Use `ba ready` to see current state.

**Note:** ba does not detect conflicting edits automatically. The line-based format makes conflicts **obvious** (entire issue on one line) but not **automatic** to resolve. Consider this when coordinating multiple agents.

## Environment and SESSION_ID

### Required: SESSION_ID

The `$SESSION_ID` environment variable **must be set** for ownership operations (claim, mine, comment with --author).

**Verification:**
```bash
# Check if SESSION_ID is set
echo ${SESSION_ID:-(not set - operations will fail)}
```

**Claude Code provides this automatically in active sessions.** If commands fail with "SESSION_ID not set":

```bash
# Generate and export a session ID
export SESSION_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
```

**Why required?** ba's ownership model prevents multi-agent conflicts by requiring explicit session identification. Without it:
- `ba claim` will fail (cannot establish ownership)
- `ba mine` will fail (cannot identify your issues)
- Multi-agent coordination breaks down

### Error Handling

If SESSION_ID is not set, ba commands will fail with:
```text
ERROR: --session is required
```

This is intentional - ownership without identity doesn't make sense.

## Known Limitations

### Stale Ownership

If a Claude Code session crashes mid-task, the issue remains `in_progress` with a stale owner. ba currently has no automatic timeout/reclaim mechanism.

**Workaround for the original owner:**
```bash
ba release <id>   # If you were the owner (SESSION_ID matches)
```

**Workaround for a different agent (manual intervention required):**
```bash
# 1. View the issue to see who owns it
ba show <id>

# 2. Manually edit .ba/issues.jsonl to remove owner/claimed_at fields
# Find the line with the issue ID and set:
#   "status": "open"
#   "owner": null
#   "claimed_at": null

# 3. Now you can claim it
ba claim <id> --session $SESSION_ID
```

**Important:** Manual JSON editing is fragile. Validate syntax after editing:
```bash
jq empty .ba/issues.jsonl  # Exit 0 = valid, non-zero = syntax error
```

**Future consideration:** Add `ba reclaim --force <id> --session $SESSION_ID` command to handle this safely.

## Quick Reference Card

```text
ba ready                            # See available work
ba claim <id> --session $SESSION_ID # Take ownership
ba show <id>                        # Check details
ba mine --session $SESSION_ID       # See your work
ba finish <id>                      # Complete
ba release <id>                     # Abandon
ba comment <id> "msg" --author $SESSION_ID
ba create "title" -t task -p 1      # New issue
ba list [--all] [--status <status>] # Browse
```
