# hi.md Plugin

hi.md is a voice-first `/hi` companion for Claude Code.

## What it does

Type `/hi`, speak naturally, and himd will:
1. Capture your voice from the microphone
2. Analyze both your words and vocal signals (energy, pace, pauses)
3. Generate a warm, context-aware reply
4. Speak the reply aloud

## Prerequisites

- **macOS** with Command Line Tools installed, **or Windows** with Visual Studio Build Tools
- **Claude Code** CLI
- **Rust** 1.88+ (for building from source)

## Install from source

```bash
git clone https://github.com/tpiperatgod/hi.md.git
cd himd
cargo build --release
```

- macOS: binary at `target/release/himd`
- Windows: binary at `target\release\himd.exe`

## Install from release artifact

If a binary has been published for your platform, download it from [GitHub Releases](https://github.com/tpiperatgod/hi.md/releases). Release artifacts are published manually and may lag the current branch state:

**macOS:**
- Apple Silicon Mac → `himd-darwin-arm64.tar.gz`
- Intel Mac → `himd-darwin-x64.tar.gz`

```bash
tar xzf himd-darwin-*.tar.gz
chmod +x himd
# Remove macOS quarantine if needed:
xattr -d com.apple.quarantine himd
```

**Windows:**
- `himd-windows-x64.zip`

```powershell
Expand-Archive .\himd-windows-x64.zip -DestinationPath .\himd
```

## Register the MCP server

The MCP server name is **`voice-bridge`** (unchanged from the Node era).

### Source install (project scope)

**macOS/Linux:**
```bash
claude mcp add --scope project --transport stdio -e DASHSCOPE_API_KEY=your-dashscope-key voice-bridge -- $(pwd)/target/release/himd serve-stdio
```

**Windows:**
```powershell
claude mcp add --scope project --transport stdio -e DASHSCOPE_API_KEY=your-dashscope-key voice-bridge -- C:\path\to\himd\target\release\himd.exe serve-stdio
```

### Release install (user scope)

**macOS/Linux:**
```bash
claude mcp add --scope user --transport stdio -e DASHSCOPE_API_KEY=your-dashscope-key voice-bridge -- /path/to/himd serve-stdio
```

**Windows:**
```powershell
claude mcp add --scope user --transport stdio -e DASHSCOPE_API_KEY=your-dashscope-key voice-bridge -- C:\path\to\himd.exe serve-stdio
```

Or use the guided setup inside Claude Code:
```
/himd:setup
```

### Migrating from legacy Node registration

If you previously registered the Node.js `@himd/voice-bridge` package, remove it and re-register with the Rust binary:

```bash
# Remove the old registration
claude mcp remove voice-bridge

# Re-register with the Rust binary (see commands above)
/himd:setup
```

The MCP server name remains `voice-bridge` so existing Claude sessions continue to work.

## Configure API key

**macOS/Linux:**
```bash
export DASHSCOPE_API_KEY="your-key"
# Add to ~/.zshrc to persist
```

**Windows (PowerShell):**
```powershell
$env:DASHSCOPE_API_KEY = "your-key"
# Add to $PROFILE to persist
```

Get your key at [DashScope](https://dashscope.console.aliyun.com/apiKey).

## Verify setup

```bash
himd doctor           # human-readable output
himd doctor --json    # machine-readable output (used by /himd:doctor)
```

Or inside Claude Code:
```
/himd:doctor
```

## Use `/hi`

1. Open Claude Code
2. Type `/hi`
3. Speak when prompted
4. Listen to the reply

## Plugin commands

| Command | Description |
|---------|-------------|
| `/hi` | Start a voice conversation |
| `/himd:setup` | Guided setup for the MCP server (source or release install) |
| `/himd:doctor` | Diagnose setup and runtime issues |

## Troubleshooting

- **"voice-bridge tools unavailable"** — run `/himd:setup` to register the MCP server
- **Missing `DASHSCOPE_API_KEY`** — set it in your shell profile and restart Claude Code
- **Binary not found** — source: run `cargo build --release`; release: re-download the artifact
- **Quarantine error on macOS** — run `xattr -d com.apple.quarantine /path/to/himd`
- **Legacy Node registration** — if `/himd:doctor` shows a registration pointing to `npx @himd/voice-bridge`, remove it and re-register with the Rust binary
- **Setup seems broken** — run `/himd:doctor` for guided diagnosis

## Local verification

This repository does not maintain GitHub Actions workflows. Verify changes locally before publishing plugin docs or preparing a manual release:

```bash
cargo test --workspace -- --test-threads=1
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
```
