# ba Codex Skill

This directory contains the Codex skill definition for ba task tracking.

## What is a Codex Skill?

Codex is Claude Code's skill system. Skills provide structured prompts that Claude can invoke during coding sessions to access specialized capabilities.

The ba skill (`$ba`) gives Claude direct access to task-tracking commands with proper context about ownership-based workflows and multi-agent coordination.

## Files

- **SKILL.md** - Main skill definition, describes available commands and workflows
- **AGENTS.md.snippet** - Comprehensive guidance for AGENTS.md files in projects using ba
- **README.md** - This file

## Installation

The Codex skill is automatically installed when you run `/ba init` in a project.

### Step 1: Install the ba Plugin

Install directly from GitHub:
```bash
claude plugin marketplace add https://github.com/cloud-atlas-ai/ba
claude plugin install ba@ba
```

Or install from local clone:
```bash
git clone https://github.com/cloud-atlas-ai/ba.git
cd ba
claude plugin marketplace add $PWD
claude plugin install ba@ba
```

### Step 2: Initialize in Your Project

```bash
/ba init
```

This installs the ba binary, runs `ba init`, downloads the `$ba` Codex skill files to `~/.codex/skills/ba/`, and updates `AGENTS.md`.

## Usage in Claude Code

Once installed, use the `$ba` prefix to invoke ba commands:

```
$ba ready           # See available work
$ba claim <id>      # Claim an issue
$ba mine            # Your claimed issues
$ba finish <id>     # Complete work
```

Claude will execute the appropriate ba commands with proper session management.

## Adding to Your Project

To document ba usage for Claude in your project:

1. Create or update your `AGENTS.md` file
2. Add the section from `AGENTS.md.snippet`:
   ```bash
   tail -n +5 ~/.codex/skills/ba/AGENTS.md.snippet >> AGENTS.md
   ```

Or use the plugin's init command:
```bash
/ba init
```

This automatically updates AGENTS.md with ba guidance.

## Design Philosophy

The skill is designed around ba's ownership-based state machine:

- **Explicit ownership**: Every in-progress issue has a known owner
- **Clear transitions**: claim → work → finish/release
- **Multi-agent safe**: Session IDs prevent conflicts
- **Dependency-aware**: `ba ready` only shows unblocked work

This makes task coordination explicit and prevents agents from stepping on each other's work.

## Relationship to Plugin

```
ba/
├── plugin/                    # Claude Code plugin
│   ├── .claude-plugin/
│   │   └── plugin.json       # Plugin manifest
│   ├── commands/
│   │   └── init.md           # /ba init command
│   └── scripts/              # Hook scripts (if any)
└── codex-skill/              # Codex skill
    ├── SKILL.md              # $ba skill definition
    ├── AGENTS.md.snippet     # Project documentation
    └── README.md             # This file
```

The plugin provides:
- Installation and setup commands
- Project initialization
- Optional hooks for automation

The skill provides:
- Task tracking commands during coding sessions
- Workflow guidance for Claude
- Session-aware operations

## Examples

### Starting a Session

```
User: Let's work on some tasks
Claude: Let me check what's available
  [Runs: ba ready]

  Here are the ready tasks:
  - ab-x7k2 (P1, task): Fix auth bug
  - ab-y8m3 (P2, feature): Add dashboard

  Which would you like to work on?
User: The auth bug
Claude: [Runs: ba claim ab-x7k2 --session $SESSION_ID]
  Claimed ab-x7k2. Let me check the details...
  [Runs: ba show ab-x7k2]
```

### Completing Work

```
Claude: I've fixed the auth bug and all tests pass. Let me mark it complete.
  [Runs: ba comment ab-x7k2 "Fixed token validation, tests passing" --author $SESSION_ID]
  [Runs: ba finish ab-x7k2]

  ✓ Completed ab-x7k2

  Would you like me to pick up another task from the ready queue?
```

## See Also

- [ba README](../README.md) - Full ba documentation
- [Claude Code Plugins](https://docs.claude.ai/plugins) - Plugin system docs
- [Codex Skills](https://docs.claude.ai/codex) - Skill system docs
