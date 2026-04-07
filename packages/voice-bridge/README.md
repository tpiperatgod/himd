# @himd/voice-bridge

Local stdio MCP server for himd voice capture, audio understanding, and TTS playback.

## What it provides

Three MCP tools for voice interaction:

- **`audio_capture_once`** — Capture audio from the microphone with VAD-based auto-stop
- **`audio_analyze`** — Transcribe speech and analyze acoustic features (energy, pace, pauses) with enriched model understanding via Qwen Omni
- **`speech_say`** — Convert text to speech via Qwen TTS and play it aloud

## Requirements

- **macOS** with Command Line Tools
- **ffmpeg** — `brew install ffmpeg`
- **Node.js** >= 20.0.0
- **`DASHSCOPE_API_KEY`** — required for both audio understanding (Qwen Omni) and spoken replies (Qwen TTS)

## Quick start with Claude Code

Register as an MCP server with environment variables inline:

```bash
claude mcp add \
  -e DASHSCOPE_API_KEY=your-dashscope-key \
  voice-bridge \
  -- npx -y @himd/voice-bridge
```

For project-local registration (adds to `.mcp.json`):

```bash
claude mcp add --scope project \
  -e DASHSCOPE_API_KEY=your-dashscope-key \
  voice-bridge \
  -- npx -y @himd/voice-bridge
```

Verify:

```bash
claude mcp list  # should show voice-bridge
```

Or inside Claude Code, type `/mcp`.

### Optional environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `AUDIO_MODEL` | `qwen3-omni-flash` | Qwen Omni model for audio understanding |
| `TTS_MODEL` | `qwen3-tts-instruct-flash` | Qwen TTS model for speech synthesis |
| `DASHSCOPE_BASE_URL` | `https://dashscope.aliyuncs.com/compatible-mode/v1` | DashScope API base URL |

Pass them with extra `-e` flags:

```bash
claude mcp add \
  -e DASHSCOPE_API_KEY=your-key \
  -e AUDIO_MODEL=qwen3-omni-flash \
  -e TTS_MODEL=qwen3-tts-instruct-flash \
  voice-bridge \
  -- npx -y @himd/voice-bridge
```

## Install (alternative)

If you prefer a global install:

```bash
npm install -g @himd/voice-bridge
claude mcp add -e DASHSCOPE_API_KEY=your-dashscope-key voice-bridge -- himd-voice-bridge
```

## Troubleshooting

- **"Required command not found: ffmpeg"** — Run `brew install ffmpeg`
- **"Required command not found: afplay"** — macOS includes afplay by default. Install Command Line Tools: `xcode-select --install`
- **Missing `DASHSCOPE_API_KEY`** — Set it in your environment; both audio understanding and spoken replies require it
- **Server not appearing in `/mcp`** — Verify registration with `claude mcp list` and restart Claude Code
