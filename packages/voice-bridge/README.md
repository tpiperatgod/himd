# @himd/voice-bridge

Local stdio MCP server for himd voice capture, audio understanding, and TTS playback.

## What it provides

Three MCP tools for voice interaction:

- **`audio_capture_once`** — Capture audio from the microphone with VAD-based auto-stop
- **`audio_analyze`** — Transcribe speech and analyze acoustic features (energy, pace, pauses) with optional enriched model understanding
- **`speech_say`** — Convert text to speech and play it aloud

## Requirements

- **macOS** with Command Line Tools
- **ffmpeg** — `brew install ffmpeg`
- **Node.js** >= 20.0.0
- **`ZHIPU_API_KEY` is required** for spoken replies via GLM-TTS
- **`DASHSCOPE_API_KEY` is optional but recommended** for Qwen Omni enriched audio understanding
- If you do not set `DASHSCOPE_API_KEY`, set `HIMD_AUDIO_PROVIDER=glm-asr` or rely on the default fallback behavior

## Quick start with Claude Code

Register as an MCP server with environment variables inline:

```bash
claude mcp add \
  -e ZHIPU_API_KEY=your-zhipu-key \
  -e DASHSCOPE_API_KEY=your-dashscope-key \
  voice-bridge \
  -- npx -y @himd/voice-bridge
```

For project-local registration (adds to `.mcp.json`):

```bash
claude mcp add --scope project \
  -e ZHIPU_API_KEY=your-zhipu-key \
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
| `HIMD_AUDIO_PROVIDER` | `qwen-omni` | Audio understanding provider (`qwen-omni` or `glm-asr`) |
| `HIMD_FALLBACK_PROVIDER` | `glm-asr` | Fallback provider when primary fails (`none` to disable) |
| `DASHSCOPE_BASE_URL` | `https://dashscope.aliyuncs.com/compatible-mode/v1` | DashScope API base URL |
| `QWEN_OMNI_MODEL` | `qwen3-omni-flash` | Qwen Omni model name |

Pass them with extra `-e` flags:

```bash
claude mcp add \
  -e ZHIPU_API_KEY=your-key \
  -e DASHSCOPE_API_KEY=your-key \
  -e HIMD_AUDIO_PROVIDER=qwen-omni \
  voice-bridge \
  -- npx -y @himd/voice-bridge
```

## Install (alternative)

If you prefer a global install:

```bash
npm install -g @himd/voice-bridge
claude mcp add -e ZHIPU_API_KEY=your-zhipu-key voice-bridge -- himd-voice-bridge
```

## Troubleshooting

- **"Required command not found: ffmpeg"** — Run `brew install ffmpeg`
- **"Required command not found: afplay"** — macOS includes afplay by default. Install Command Line Tools: `xcode-select --install`
- **Missing `ZHIPU_API_KEY`** — Set it in your environment; spoken replies require it
- **Missing `DASHSCOPE_API_KEY`** — Set it to enable Qwen Omni, or use `HIMD_AUDIO_PROVIDER=glm-asr`
- **Provider fallback fails** — Both providers may be unavailable. Check your API keys and network connectivity
- **Server not appearing in `/mcp`** — Verify registration with `claude mcp list` and restart Claude Code
