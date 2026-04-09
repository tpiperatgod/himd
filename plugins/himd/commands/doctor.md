---
description: "Diagnose himd setup and runtime issues"
---

# /himd:doctor

You are running a two-layer diagnostic for the himd voice companion. Follow the steps **in order** and present **one primary conclusion** with **one recommended fix**.

## Layer 1 — Local capability diagnostics

Run the Rust binary's diagnostics:

```bash
<BINARY_PATH> doctor --json
```

To find the binary path, check the `voice-bridge` registration:

```bash
claude mcp get voice-bridge
```

If `voice-bridge` is not registered or the binary path cannot be determined, ask the user where they installed the `himd` binary. Common locations:

- **macOS/Linux**: `~/.local/bin/himd`
- **Windows**: `%LOCALAPPDATA%\himd\himd.exe`

If the binary is not installed at all, direct the user to `/himd:setup`.

Parse the JSON output. Examine:

- `readiness.pass` — overall pass/fail
- `readiness.failure_codes` — list of specific failures
- `config.dashscope_api_key_set` — API key presence
- `audio_input.available` — microphone detected
- `audio_output.available` — speaker detected
- `runtime_state.writable` — state directory writable
- `binary.platform`, `binary.architecture` — verify they match the local machine

### Failure code reference

| Code | Meaning | Fix |
|------|---------|-----|
| `binary_arch_mismatch` | Binary compiled for wrong architecture | Download the correct artifact (macOS: `arm64` or `x64`; Windows: `x64`) |
| `binary_quarantined` | macOS quarantined the downloaded binary | `xattr -d com.apple.quarantine /path/to/himd` |
| `binary_missing` | Binary not found at the expected path | Re-run `/himd:setup` to download and install the binary |
| `binary_not_executable` | Binary exists but is not executable | macOS/Linux: `chmod +x /path/to/himd` |
| `dashscope_api_key_missing` | `DASHSCOPE_API_KEY` not set | Set it in your shell profile and restart Claude Code |
| `audio_input_missing` | No microphone detected | Check audio input settings (macOS: System Settings > Sound > Input; Windows: Settings > System > Sound > Input) |
| `audio_input_init_failed` | Microphone found but failed to initialize | Check microphone permissions |
| `audio_output_missing` | No speaker detected | Check audio output settings |
| `audio_output_init_failed` | Speaker found but failed to initialize | Try reconnecting the audio device |
| `runtime_state_dir_not_writable` | Runtime state directory is not writable | Check directory permissions |

## Layer 2 — Claude MCP registration diagnostics

Only run this layer **if local capability passes** (`readiness.pass` is `true`). If local capability fails, stop here — fixing local issues comes first.

### Check 1: Registration exists

```bash
claude mcp get voice-bridge
```

- If `voice-bridge` is not registered → conclusion is: **registration missing**
- If registered, record the scope and command path.

### Check 2: Legacy Node registration detection

If the registered command contains `npx`, `node`, or `@himd/voice-bridge`, this is a **legacy Node.js registration** that must be migrated:

> The `voice-bridge` registration is a legacy Node.js entry (`npx @himd/voice-bridge`). The Rust binary is the current production path. Fix: remove the old registration and re-register with the Rust binary via `/himd:setup`.

### Check 3: Wrong path

Verify the registered command points to a `himd` binary that actually exists:

- macOS/Linux: `ls -la <path>`
- Windows: `Test-Path <path>`

If the binary is missing → conclusion includes `binary_missing`.

### Check 4: Conflicting scopes

```bash
claude mcp list
```

Check if `voice-bridge` appears in multiple scopes. If so, this is a conflict — remove all entries and re-register with the correct scope.

## Present the conclusion

Reduce all findings into **one** of these conclusions:

### Conclusion A: Local capability is not ready

> The himd binary reports that local capabilities are not ready. The issue is: **[describe the specific failure]**.
>
> Fix: **[specific fix from the failure code table above]**
>
> After fixing, re-run `/himd:doctor` to verify.

### Conclusion B: Legacy Node registration detected

> The `voice-bridge` MCP registration is a legacy Node.js entry. The Rust binary is the current production path.
>
> Fix:
> 1. Remove the legacy registration: `claude mcp remove voice-bridge`
> 2. Re-register by running `/himd:setup`

### Conclusion C: Local capability ready, but MCP registration is missing

> The himd binary is healthy, but `voice-bridge` is not registered with Claude Code.
>
> Fix: Run `/himd:setup` to register the MCP server.

### Conclusion D: Local capability ready, but MCP registration is stale or broken

> The himd binary is healthy, but the `voice-bridge` registration has an issue: **[describe: wrong path, wrong scope, conflicting scopes, missing env var]**.
>
> Fix:
> 1. Remove the stale registration: `claude mcp remove voice-bridge`
> 2. Re-register by running `/himd:setup`

### Conclusion E: System is ready

> All diagnostics pass. The himd binary is healthy and `voice-bridge` is correctly registered.
>
> You can start a voice conversation with `/hi`.

## Rules

- **Always** run Layer 1 before Layer 2.
- **Never** silently remove or modify Claude MCP configuration. Always present the action and let the user confirm.
- **Never** present a long unordered checklist. Give one conclusion and one fix.
