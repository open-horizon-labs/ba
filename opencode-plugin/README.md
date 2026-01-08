# ba OpenCode Plugin

TypeScript plugin for running ba task tracking with [OpenCode](https://opencode.ai).

## Overview

This plugin enables ba's ownership-based task tracking for OpenCode users. It provides tool commands for initialization, status checking, and quick reference.

## Installation

### Option A: Download from GitHub Release (easiest)

```bash
cd /path/to/your/project

# 1. Download pre-built plugin
curl -L -o ba.js https://github.com/cloud-atlas-ai/ba/releases/latest/download/opencode-index.js

# 2. Install plugin
mkdir -p .opencode/plugin
mv ba.js .opencode/plugin/

# 3. Start OpenCode
opencode

# 4. Initialize ba by asking OpenCode to use the 'ba-init' tool
# For example: "use the ba-init tool to initialize the project"
```

### Option B: Build from source

```bash
# 1. Clone and build
git clone https://github.com/cloud-atlas-ai/ba.git /tmp/ba
cd /tmp/ba/opencode-plugin
bun install
bun build src/index.ts --outdir dist --target bun

# 2. Install plugin
cd /path/to/your/project
mkdir -p .opencode/plugin
cp /tmp/ba/opencode-plugin/dist/index.js .opencode/plugin/ba.js

# 3. Start OpenCode and ask it to initialize ba
opencode
```

### Option C: Global install

```bash
# 1. Download or build plugin
curl -L -o ba.js https://github.com/cloud-atlas-ai/ba/releases/latest/download/opencode-index.js

# 2. Install globally
mkdir -p ~/.config/opencode/plugin
mv ba.js ~/.config/opencode/plugin/

# 3. In each project, ask OpenCode to initialize ba
```

## Tool Commands

The plugin provides three tools:

| Tool | Description |
|------|-------------|
| `ba-init` | Initialize ba, checks for binary, guides installation if needed |
| `ba-status` | Show project status, issue counts, and your claimed issues |
| `ba-quickstart` | Display quick reference guide |

### Usage Examples

**Initialize ba:**
```
"use the ba-init tool to set up ba for this project"
```

**Check status:**
```
"check ba status"
```

**Get quick reference:**
```
"show ba quickstart"
```

## What ba-init Does

1. **Checks if already initialized** - Shows current status if .ba/ exists
2. **Checks for ba binary** - Verifies ba is installed
3. **Guides installation if needed** - Detects Homebrew or Cargo, provides installation commands
4. **Initializes project** - Runs `ba init` to create .ba/ directory
5. **Updates AGENTS.md** - Adds ba workflow guidance (creates file if needed)

## Architecture

Unlike the Claude Code plugin which uses slash commands and Codex skills, the OpenCode plugin:
- Uses **tool commands** instead of slash commands
- No Codex skill (OpenCode doesn't have Codex)
- Direct ba binary integration via shell commands
- Same .ba/ directory structure as other ba clients

## Compatibility

**Shared with Claude Code plugin:**
- `.ba/` directory structure
- `ba` binary and CLI commands
- AGENTS.md documentation format
- Workflow and state machine

**Different from Claude Code plugin:**
- Tool-based instead of slash commands
- No Codex skill installation
- TypeScript instead of shell scripts

## Requirements

- OpenCode configured with an LLM provider
- ba binary (plugin guides installation if missing)
  - Via Homebrew: `brew install cloud-atlas-ai/ba/ba`
  - Via Cargo: `cargo install ba`

## Configuration

No configuration needed. The plugin:
- Detects available package managers
- Uses ba binary from PATH
- Reads/writes to `.ba/` in project directory
- Respects `$SESSION_ID` environment variable for ownership

## Development

```bash
cd opencode-plugin
bun install
bun run typecheck  # Check types
bun build src/index.ts --outdir dist --target bun
```

## Testing

After installation:

1. **Plugin loads**: Start OpenCode in any directory
2. **Initialize**: Ask OpenCode to "use ba-init tool"
3. **Check status**: Ask OpenCode to "check ba status"
4. **View reference**: Ask OpenCode to "show ba quickstart"

## Troubleshooting

**"ba binary not found"**: The plugin will guide you to install via Homebrew or Cargo

**"ba not initialized"**: Run ba-init tool first

**No SESSION_ID**: OpenCode should provide this automatically. If not, some ownership features may not work.

## See Also

- [ba README](../README.md) - Main ba documentation
- [Claude Code plugin](../plugin/README.md) - Claude Code version
- [ba CLI](../README.md#commands) - Full command reference

## Philosophy

ba provides **ownership-based task tracking** for multi-agent workflows:

- **Explicit ownership**: Every in-progress issue has a known owner
- **State transitions**: claim → work → finish/release
- **Multi-agent safe**: Session IDs prevent conflicts
- **Dependency-aware**: Only show unblocked work

This plugin makes ba's workflow first-class in OpenCode with easy setup and status visibility.
