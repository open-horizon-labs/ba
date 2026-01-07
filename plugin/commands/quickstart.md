# ba Quick Start

Show a quick start guide for using ba in this session.

## Display Guide

Show this concise reference:

```text
ba - Task Tracking Quick Start
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

More info: ba --help or README.md
```

## Check if initialized

If `.ba/` doesn't exist, suggest:
```text
ba not initialized yet. Run:
  /ba init
```

---
Keep it concise. This is a reference card, not a tutorial.
