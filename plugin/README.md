# ba Claude Code Plugin

Claude Code plugin for ba task-tracking integration.

## What This Plugin Provides

### Slash Commands

- `/ba init` - Initialize ba for a project, install binary if needed, update CLAUDE.md
- `/ba status` - Show current ba status, issue counts, your claimed issues
- `/ba quickstart` - Display quick reference guide

### Codex Skill

The `$ba` skill for Codex-enabled sessions:
- `$ba ready` - See available work
- `$ba claim <id>` - Take ownership
- `$ba mine` - Your claimed issues
- `$ba finish <id>` - Complete work
- `$ba show <id>` - Check details

See [codex-skill/SKILL.md](../codex-skill/SKILL.md) for full skill documentation.

**Note:** The `$ba` skill requires `$SESSION_ID` environment variable. Claude Code typically provides this automatically. See SKILL.md for details on SESSION_ID requirements and error handling.

### Project Integration

The init command automatically:
1. Installs the ba binary (via Homebrew or Cargo)
2. Runs `ba init` to create `.ba/` directory
3. Updates `AGENTS.md` with ba workflow guidance

## Installation

### From Claude Code

```bash
claude plugin install ba
```

### From Source (Development)

```bash
cd /path/to/ba
claude plugin install plugin/
```

## Usage

### Initializing a Project

In any project directory:

```bash
/ba init
```

This will:
- Check if ba is already initialized
- Install ba binary if not available (offers Homebrew or Cargo)
- Run `ba init` to create `.ba/` structure
- Update CLAUDE.md with ba guidance

### Checking Status

```bash
/ba status
```

Shows:
- Project configuration
- Issue counts by status
- Ready queue summary
- Your claimed issues (if `$SESSION_ID` available)

### Quick Reference

```bash
/ba quickstart
```

Displays a concise command reference card.

## Project Structure

```text
plugin/
├── .claude-plugin/
│   └── plugin.json       # Plugin manifest
├── commands/
│   ├── init.md           # /ba init command
│   ├── status.md         # /ba status command
│   └── quickstart.md     # /ba quickstart command
├── scripts/              # Hook scripts (future)
└── README.md             # This file
```

## How It Works

### Slash Commands

Slash commands are markdown files in `commands/` that Claude Code interprets as instructions. When you run `/ba init`, Claude reads `commands/init.md` and executes the steps.

Each command file:
- Describes the goal
- Provides step-by-step instructions
- Includes shell commands to run
- Handles edge cases and errors

### Codex Skills

The `$ba` skill is defined in `../codex-skill/SKILL.md`. Codex skills:
- Provide structured prompts
- Include command examples
- Document workflows and patterns
- Are available in Codex-enabled sessions

Claude invokes the appropriate ba commands with proper session management.

## Development

### Testing Commands

Test a command locally:

```bash
# Read the command file to understand what it does
cat commands/init.md

# Run the steps manually or via Claude Code
claude code
> /ba init
```

### Adding New Commands

1. Create a new `.md` file in `commands/`
2. Follow the pattern from existing commands
3. Be concise and actionable
4. Test thoroughly

### Updating the Plugin

After making changes:

```bash
# Update version in plugin.json
vim .claude-plugin/plugin.json

# Reinstall for testing
claude plugin uninstall ba
claude plugin install plugin/
```

## Integration with ba Binary

This plugin wraps the `ba` binary, which must be installed separately. The plugin helps with installation but doesn't include the binary itself.

Binary installation options:
- **Homebrew**: `brew install cloud-atlas-ai/ba/ba`
- **Cargo**: `cargo install ba`
- **From source**: `cargo install --path /path/to/ba`

See the main [ba README](../README.md) for binary documentation.

## Relationship to Codex Skill

```text
ba/
├── plugin/                # This directory
│   └── commands/          # Slash commands (/ba init, /ba status)
└── codex-skill/           # Separate directory
    └── SKILL.md           # Codex skill ($ba ready, $ba claim)
```

**Plugin**: Setup, configuration, project initialization
**Skill**: Day-to-day task tracking during coding sessions

Both work together to integrate ba into Claude Code workflows.

## Philosophy

ba provides **ownership-based task tracking** for multi-agent workflows:

- **Explicit ownership**: Every in-progress issue has a known owner
- **State transitions**: claim → work → finish/release
- **Multi-agent safe**: Session IDs prevent conflicts
- **Dependency-aware**: Only show unblocked work

This plugin makes ba's workflow first-class in Claude Code, with:
- Easy setup via `/ba init`
- Status visibility via `/ba status`
- Direct command access via `$ba` skill

## Examples

### First-Time Setup

```text
User: Let's use ba for task tracking
Claude: I'll initialize ba for this project
  [Runs: /ba init]

  Checking for ba binary...
  Not found. I can install via Homebrew or Cargo.

  [Shows installation options]

User: Use Homebrew
Claude: [Runs: brew install cloud-atlas-ai/ba/ba]
  [Runs: ba init]
  [Updates AGENTS.md]

  ✓ ba initialized and ready

  Quick start:
    ba create "Your first task" -t task
    ba list
```

### During a Session

```text
Claude: Let me check what tasks are available
  [Uses: $ba ready]

  3 issues ready:
  - ab-x7k2 (P1): Fix auth bug
  - ab-y8m3 (P2): Add dashboard
  - ab-z9n4 (P3): Update docs

User: Let's fix the auth bug
Claude: [Uses: $ba claim ab-x7k2]
  [Uses: $ba show ab-x7k2]

  Claimed ab-x7k2. Details:
  - Created 2 days ago
  - Priority: 1 (High)
  - Description: Token validation fails for special chars

  Let me investigate...
```

## See Also

- [ba README](../README.md) - Main ba documentation
- [Codex Skill](../codex-skill/README.md) - Skill documentation
- [Claude Code Plugins](https://docs.claude.ai/plugins) - Plugin system
