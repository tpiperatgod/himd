# himd

himd = hi.md, a voice-first `/hi` companion for Claude Code.

himd is distributed as:

- **a Claude Code plugin** in `plugins/himd/` — the `/hi` command
- **a local MCP server package** in `packages/voice-bridge/` — audio capture, understanding, and playback

## Quick links

- **End users:** start with [`plugins/himd/README.md`](plugins/himd/README.md) for install and usage
- **MCP setup:** see [`packages/voice-bridge/README.md`](packages/voice-bridge/README.md) for the voice-bridge server
- **Contributors:** `pnpm install && pnpm run check`

## What it does

Type `/hi`, speak naturally, and himd will:

1. Capture your voice from the microphone
2. Analyze both your words and vocal signals (energy, pace, pauses, emotion, tone)
3. Generate a warm, context-aware Chinese reply
4. Speak the reply aloud

Instead of `speech -> text -> response`, himd moves closer to `speech -> understanding (content + state) -> interaction`.

## Audio providers

| Provider | Capabilities | Config name |
|----------|-------------|-------------|
| Qwen Omni | Transcript + emotion + intent + tone + summary | `qwen-omni` |
| GLM-ASR | Transcript only | `glm-asr` |

Automatic fallback: `qwen-omni` (primary) -> `glm-asr` (fallback). Disable with `HIMD_FALLBACK_PROVIDER=none`.

## Configuration

See [packages/voice-bridge/README.md](packages/voice-bridge/README.md) for environment variable reference.

## Project structure

```text
plugins/himd/                     # Claude Code plugin
  .claude-plugin/plugin.json      # plugin manifest
  commands/hi.md                  # /hi command
packages/voice-bridge/            # MCP server (npm package)
  bin/himd-voice-bridge.js        # CLI entry
  index.js                        # MCP tool definitions
  capture.js                      # microphone capture with auto-stop
  analyze.js                      # acoustic analysis + audio_turn builder
  tts.js                          # speech synthesis and playback
  providers/                      # audio understanding providers
  prompts/                        # model prompt templates
scripts/                          # validation and sync scripts
```

## Limitations

- macOS only (uses `ffmpeg` for capture, `afplay` for playback)
- Audio file size capped at 25MB, duration at 30 seconds
- Only `.wav` and `.mp3` formats supported

## License

MIT
