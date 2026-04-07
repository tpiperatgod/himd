const fs = require("fs");
const path = require("path");
const { McpServer } = require("@modelcontextprotocol/sdk/server/mcp.js");
const { StdioServerTransport } = require("@modelcontextprotocol/sdk/server/stdio.js");
const { z } = require("zod");
const { analyzeAudio, buildAudioTurn } = require("./analyze.js");
const { synthesize, playAudio, markSpeechTurn, readProfile, writeProfile } = require("./tts.js");
const { captureOnce, getControlDir } = require("./capture.js");
const qwenOmni = require("./providers/qwen-omni-provider.js");

const server = new McpServer({
  name: "voice-bridge",
  version: "0.5.0",
});

// --- Audio understanding via Qwen Omni ---

const MAX_FILE_SIZE = 25 * 1024 * 1024; // 25 MB
const SUPPORTED_EXTENSIONS = [".wav", ".mp3"];

server.tool(
  "audio_transcribe",
  "Transcribe a local audio file using Qwen Omni. Returns the transcript text.",
  {
    file_path: z.string().describe("Absolute path to the audio file (.wav or .mp3, max 25MB, max 30 seconds)"),
  },
  async ({ file_path }) => {
    if (!fs.existsSync(file_path)) {
      return {
        content: [{ type: "text", text: JSON.stringify({ error: "File not found", file_path }) }],
        isError: true,
      };
    }

    const ext = file_path.toLowerCase().slice(file_path.lastIndexOf("."));
    if (!SUPPORTED_EXTENSIONS.includes(ext)) {
      return {
        content: [{
          type: "text",
          text: JSON.stringify({ error: `Unsupported format: ${ext}. Use .wav or .mp3`, file_path }),
        }],
        isError: true,
      };
    }

    const stat = fs.statSync(file_path);
    if (stat.size > MAX_FILE_SIZE) {
      return {
        content: [{
          type: "text",
          text: JSON.stringify({ error: `File too large: ${(stat.size / 1024 / 1024).toFixed(1)}MB (max 25MB)`, file_path }),
        }],
        isError: true,
      };
    }

    try {
      const result = await qwenOmni.understand(file_path);
      return {
        content: [{
          type: "text",
          text: JSON.stringify({
            transcript: result.transcript,
            source: "file",
            audio_file: file_path,
            model: result.model,
          }),
        }],
      };
    } catch (err) {
      return {
        content: [{ type: "text", text: JSON.stringify({ error: err.message, file_path }) }],
        isError: true,
      };
    }
  }
);

// --- Audio analysis (provider understanding + acoustic features) ---

server.tool(
  "audio_analyze",
  "Analyze a local audio file: transcribe via ASR and extract acoustic features (speech_rate, energy, pause_pattern). Returns a structured audio_turn.",
  {
    file_path: z.string().describe("Absolute path to the audio file (.wav or .mp3, max 25MB, max 30 seconds)"),
  },
  async ({ file_path }) => {
    if (!fs.existsSync(file_path)) {
      return {
        content: [{ type: "text", text: JSON.stringify({ error: "File not found", file_path }) }],
        isError: true,
      };
    }

    const ext = file_path.toLowerCase().slice(file_path.lastIndexOf("."));
    if (!SUPPORTED_EXTENSIONS.includes(ext)) {
      return {
        content: [{
          type: "text",
          text: JSON.stringify({ error: `Unsupported format: ${ext}. Use .wav or .mp3`, file_path }),
        }],
        isError: true,
      };
    }

    const stat = fs.statSync(file_path);
    if (stat.size > MAX_FILE_SIZE) {
      return {
        content: [{
          type: "text",
          text: JSON.stringify({ error: `File too large: ${(stat.size / 1024 / 1024).toFixed(1)}MB (max 25MB)`, file_path }),
        }],
        isError: true,
      };
    }

    try {
      // Step 1: Audio understanding via Qwen Omni
      const providerResult = await qwenOmni.understand(file_path);

      // Step 2: Local acoustic analysis (supplementary)
      const rawAnalysis = analyzeAudio(file_path, providerResult.transcript);

      // Step 3: Build unified audio_turn with enriched understanding
      const audioTurn = buildAudioTurn(providerResult, rawAnalysis, file_path);

      return {
        content: [{
          type: "text",
          text: JSON.stringify(audioTurn),
        }],
      };
    } catch (err) {
      return {
        content: [{ type: "text", text: JSON.stringify({ error: err.message, file_path }) }],
        isError: true,
      };
    }
  }
);

// --- Phase 5: Auto-capture from microphone ---

server.tool(
  "audio_capture_once",
  "Capture audio from the local microphone. Recording starts immediately and stops when speech ends (VAD silence detection), no speech is detected within grace period, or max_duration_sec is reached. Returns the temp file path and metadata.",
  {
    max_duration_sec: z.number().optional().describe("Safety cap in seconds (default 30, max 60). Recording stops automatically if user doesn't press Enter."),
  },
  async ({ max_duration_sec }) => {
    try {
      const result = await captureOnce(max_duration_sec);
      return {
        content: [{
          type: "text",
          text: JSON.stringify(result),
        }],
      };
    } catch (err) {
      return {
        content: [{ type: "text", text: JSON.stringify({ error: err.message }) }],
        isError: true,
      };
    }
  }
);

// --- Stop ongoing capture ---

server.tool(
  "audio_stop_capture",
  "Stop an ongoing audio capture. Use this if the user wants to stop recording manually.",
  {},
  async () => {
    const controlDir = getControlDir();
    const stopFile = path.join(controlDir, ".stop-capture");
    const pidFile = path.join(controlDir, ".capture-pid");

    // Check if a capture is actually running
    if (!fs.existsSync(pidFile)) {
      return {
        content: [{ type: "text", text: JSON.stringify({ stopped: false, reason: "no active capture" }) }],
      };
    }

    try {
      fs.mkdirSync(controlDir, { recursive: true });
      fs.writeFileSync(stopFile, String(Date.now()));
      return {
        content: [{ type: "text", text: JSON.stringify({ stopped: true }) }],
      };
    } catch (err) {
      return {
        content: [{ type: "text", text: JSON.stringify({ stopped: false, error: err.message }) }],
        isError: true,
      };
    }
  }
);

// --- TTS voice output ---

server.tool(
  "speech_say",
  "Convert text to speech using Qwen TTS and play it. Use this after generating a reply to speak it aloud.",
  {
    text: z.string().describe("Text to speak (required, max 600 chars)"),
    voice: z.string().optional().describe("Optional Qwen TTS voice name."),
    instructions: z.string().optional().describe("Optional natural-language speaking instructions."),
    optimize_instructions: z.boolean().optional().describe("Whether DashScope should optimize the speaking instructions."),
  },
  async ({ text, voice, instructions, optimize_instructions }) => {
    try {
      const result = await synthesize(text, { voice, instructions, optimize_instructions });
      const played = playAudio(result.audioFile);
      markSpeechTurn();

      return {
        content: [{
          type: "text",
          text: JSON.stringify({
            spoken: played,
            audio_file: result.audioFile,
            model: result.model,
            voice: result.voice,
            instructions: result.instructions,
            optimize_instructions: result.optimizeInstructions,
            text_length: result.textLength,
          }),
        }],
      };
    } catch (err) {
      return {
        content: [{ type: "text", text: JSON.stringify({ spoken: false, error: err.message }) }],
        isError: true,
      };
    }
  }
);

server.tool(
  "speech_set_profile",
  "Set or update the default voice profile for TTS output.",
  {
    voice: z.string().optional().describe("Optional Qwen TTS voice name."),
    instructions: z.string().optional().describe("Optional natural-language speaking instructions."),
    optimize_instructions: z.boolean().optional().describe("Whether DashScope should optimize the speaking instructions."),
  },
  async ({ voice, instructions, optimize_instructions }) => {
    try {
      const profile = writeProfile({ voice, instructions, optimize_instructions });
      return {
        content: [{
          type: "text",
          text: JSON.stringify({ profile }),
        }],
      };
    } catch (err) {
      return {
        content: [{ type: "text", text: JSON.stringify({ error: err.message }) }],
        isError: true,
      };
    }
  }
);

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch(console.error);
