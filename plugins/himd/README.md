# himd Plugin

himd = hi.md, a voice-first `/hi` companion for Claude Code.

## What it does

Type `/hi`, speak naturally, and himd will:
1. Capture your voice from the microphone
2. Analyze both your words and vocal signals (energy, pace, pauses)
3. Generate a warm, context-aware Chinese reply
4. Speak the reply aloud

## Prerequisites

- **macOS** with Command Line Tools installed
- **ffmpeg** — install with `brew install ffmpeg`
- **Claude Code** CLI

## Install from marketplace

> Coming soon — the plugin will be available from the Claude Code plugin marketplace.

## Install and register `@himd/voice-bridge`

himd requires the `voice-bridge` MCP server for audio capture, analysis, and playback:

```bash
npm install -g @himd/voice-bridge
claude mcp add --transport stdio -e ZHIPU_API_KEY=your-zhipu-key voice-bridge -- himd-voice-bridge
```

Or use it directly without installing:

```bash
claude mcp add --transport stdio -e ZHIPU_API_KEY=your-zhipu-key voice-bridge -- npx -y @himd/voice-bridge
```

## Configure API keys

For the full `/hi` experience:

- `ZHIPU_API_KEY` is required for spoken replies
- `DASHSCOPE_API_KEY` is optional but recommended for Qwen Omni enriched understanding

```bash
export ZHIPU_API_KEY="your-key"         # Required for speech synthesis
export DASHSCOPE_API_KEY="your-key"     # Optional, enables Qwen Omni
```

If you only configure `ZHIPU_API_KEY`, himd can still work using GLM-ASR transcription plus GLM-TTS.
Add these to your shell profile (`~/.zshrc`) to persist them.

## Verify setup

```bash
claude mcp list  # should show voice-bridge
```

Or inside Claude Code, type `/mcp`.

## Use `/hi`

1. Open Claude Code
2. Type `/hi`
3. Speak when prompted
4. Listen to the reply

## Local development

To test the plugin from source:

```bash
git clone git@github.com:tpiperatgod/himd.git
cd himd
claude --plugin-dir ./plugins/himd
```

## Troubleshooting

- **"voice-bridge tools unavailable"** — run `claude mcp add --transport stdio -e ZHIPU_API_KEY=your-zhipu-key voice-bridge -- himd-voice-bridge`
- **"Required command not found: ffmpeg"** — run `brew install ffmpeg`
- **Missing `ZHIPU_API_KEY`** — set it in your environment; spoken replies require it
- **Missing `DASHSCOPE_API_KEY`** — set it to enable Qwen Omni, or use GLM-ASR only
- **Plugin not loading** — verify `plugins/himd/.claude-plugin/plugin.json` exists and is valid JSON
