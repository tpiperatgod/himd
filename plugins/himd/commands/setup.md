---
description: "Set up the himd voice companion (Rust MCP server)"
---

# /himd:setup

You are running the guided setup for the himd voice companion. Follow these stages **in order** and stop at the first problem.

## Stage 1 — Download the binary

Detect the user's platform and instruct them to download the correct artifact from GitHub Releases.

| Platform | Artifact |
|----------|----------|
| Apple Silicon Mac | `himd-darwin-arm64.tar.gz` |
| Intel Mac | `himd-darwin-x64.tar.gz` |
| Windows | `himd-windows-x64.zip` |

Provide the download URL pattern:

> Download the latest release from: `https://github.com/tpiperatgod/hi.md/releases/latest`

### Extract

**macOS/Linux:**
```bash
tar xzf himd-darwin-*.tar.gz && chmod +x himd
```

If macOS quarantine blocks execution:
```bash
xattr -d com.apple.quarantine himd
```

**Windows:**
```powershell
Expand-Archive .\himd-windows-x64.zip -DestinationPath .\himd
```

### Recommended install location

Suggest the user move the binary to a stable path:

- **macOS/Linux**: `~/.local/bin/himd`
- **Windows**: `%LOCALAPPDATA%\himd\himd.exe`

Ask the user for the **absolute path** where the `himd` binary is located. Verify it exists:

- macOS/Linux: `ls -la <path>`
- Windows: `Get-Item <path>`

Do not proceed until the binary exists at the confirmed path.

## Stage 2 — Verify local capability (gate)

Run the binary's diagnostics before attempting MCP registration:

```bash
<BINARY_PATH> doctor --json
```

Parse the JSON output. Check `readiness.pass`:

- If `pass` is `true`: proceed to Stage 3.
- If `pass` is `false`: **stop here** and route the user to `/himd:doctor`. Display the failure codes and a brief explanation:
  - `dashscope_api_key_missing` → "Set `DASHSCOPE_API_KEY` in your shell profile and restart Claude Code."
  - `audio_input_missing` → "No microphone detected. Check audio input settings."
  - `audio_output_missing` → "No speaker detected. Check audio output settings."
  - `runtime_state_dir_not_writable` → "The runtime state directory is not writable."

Do **not** continue to MCP registration until local capability passes.

## Stage 3 — Check for legacy Node registration

Before registering, check for legacy `@himd/voice-bridge` Node.js registrations:

```bash
claude mcp get voice-bridge
```

If the registered command contains `npx`, `node`, or `@himd/voice-bridge`, this is a **legacy Node registration** that must be migrated:

> A legacy Node.js `voice-bridge` registration was found. The Rust binary is the current production path. I will remove the old registration and replace it with the Rust binary.

```bash
claude mcp remove voice-bridge
```

Then proceed to register the Rust binary (below).

## Stage 4 — Register or repair the MCP server

The MCP server name is **`voice-bridge`**.

### Registration (user scope)

**macOS/Linux:**
```bash
claude mcp add --scope user --transport stdio -e DASHSCOPE_API_KEY=$DASHSCOPE_API_KEY voice-bridge -- <ABSOLUTE_PATH_TO_BINARY> serve-stdio
```

**Windows:**
```powershell
claude mcp add --scope user --transport stdio -e DASHSCOPE_API_KEY=$env:DASHSCOPE_API_KEY voice-bridge -- <ABSOLUTE_PATH_TO_BINARY> serve-stdio
```

### If registration already exists

Before adding, check the current registration:

```bash
claude mcp get voice-bridge
```

If `voice-bridge` is already registered:

- **Correct path and user scope**: no action needed, proceed to Stage 5.
- **Stale path or wrong scope**: remove and re-add:
  ```bash
  claude mcp remove voice-bridge
  claude mcp add --scope user --transport stdio -e DASHSCOPE_API_KEY=$DASHSCOPE_API_KEY voice-bridge -- <ABSOLUTE_PATH_TO_BINARY> serve-stdio
  ```
- **Multiple registrations across scopes**: warn the user, remove all conflicting entries, and re-add with user scope.

Do **not** silently remove registrations without telling the user what you are doing.

## Stage 5 — Verify the registration

```bash
claude mcp get voice-bridge
```

Confirm:
- The server name is `voice-bridge`.
- The transport is `stdio`.
- The command points to the correct `himd` binary path.
- The `DASHSCOPE_API_KEY` environment variable is set in the server config.

## Stage 6 — Direct the user

If all stages pass:

> Setup complete. You can start a voice conversation with `/hi`, or run a full diagnostic with `/himd:doctor`.

If any stage failed, direct the user to `/himd:doctor` for guided troubleshooting.
