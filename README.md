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

**Step 2 — Configure API keys:**

```bash
export DASHSCOPE_API_KEY="your-key"     # Required — Qwen Omni audio emotion/intent analysis
export ZHIPU_API_KEY="your-key"         # Required — GLM-TTS speech playback
```

Get your keys: [DashScope](https://dashscope.console.aliyun.com/apiKey) · [Zhipu AI](https://open.bigmodel.cn/usercenter/apikeys)

Add these to your shell profile (`~/.zshrc`) to persist them.

**Step 3 — Install and register the voice-bridge MCP server:**

```bash
npm install -g @himd/voice-bridge
claude mcp add --transport stdio -e DASHSCOPE_API_KEY=$DASHSCOPE_API_KEY -e ZHIPU_API_KEY=$ZHIPU_API_KEY voice-bridge -- himd-voice-bridge
```

Or without global install:

```bash
claude mcp add --transport stdio -e DASHSCOPE_API_KEY=$DASHSCOPE_API_KEY -e ZHIPU_API_KEY=$ZHIPU_API_KEY voice-bridge -- npx -y @himd/voice-bridge
```

**Step 4 — Use it:**

```
/himd:hi
```

## Audio providers

| Provider | Capabilities | Config name |
|----------|-------------|-------------|
| Qwen Omni | Transcript + emotion + intent + tone + summary | `qwen-omni` |
| GLM-ASR | Transcript only | `glm-asr` |

Automatic fallback: `qwen-omni` → `glm-asr`. Disable with `HIMD_FALLBACK_PROVIDER=none`.

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
