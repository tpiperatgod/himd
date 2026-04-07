# himd

himd = hi.md, a voice-first `/hi` companion for Claude Code.

Type `/himd:hi`, speak naturally, and himd will capture your voice, analyze both your words and vocal signals, generate a warm reply, and speak it aloud.

Instead of `speech → text → response`, himd moves closer to `speech → understanding (content + state) → interaction`.

## Install

> Requires **macOS** and **ffmpeg** (`brew install ffmpeg`).

**Step 1 — Add the marketplace and install the plugin:**

```bash
/plugin marketplace add tpiperatgod/himd
/plugin install himd@himd
```

**Step 2 — Configure API key and models:**

```bash
export DASHSCOPE_API_KEY="your-key"
export AUDIO_MODEL="qwen3-omni-flash"
export TTS_MODEL="qwen3-tts-instruct-flash"
```

Get your key: [DashScope](https://dashscope.console.aliyun.com/apiKey)

Add these to your shell profile (`~/.zshrc`) to persist them.

**Step 3 — Install and register the voice-bridge MCP server:**

```bash
npm install -g @himd/voice-bridge
claude mcp add --transport stdio -e DASHSCOPE_API_KEY=$DASHSCOPE_API_KEY voice-bridge -- himd-voice-bridge
```

Or without global install:

```bash
claude mcp add --transport stdio -e DASHSCOPE_API_KEY=$DASHSCOPE_API_KEY voice-bridge -- npx -y @himd/voice-bridge
```

**Step 4 — Use it:**

```
/himd:hi
```

## Audio understanding

Audio understanding uses Qwen Omni via DashScope, providing transcript + emotion + intent + tone + summary from a single API.

## Contributing

```bash
git clone git@github.com:tpiperatgod/himd.git
cd himd
pnpm install && pnpm run check
```

See [`packages/voice-bridge/README.md`](packages/voice-bridge/README.md) for MCP server details.

## Limitations

- macOS only (`ffmpeg` for capture, `afplay` for playback)
- Audio: max 25 MB, max 30s, `.wav` and `.mp3` only

## License

MIT
