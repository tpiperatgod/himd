const { spawn } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { assertCommandAvailable } = require("./system-deps.js");

/**
 * Resolve ffmpeg path from PATH, with common fallback locations.
 */
function findFfmpeg() {
  const { execSync } = require("child_process");
  try {
    return execSync("which ffmpeg", { encoding: "utf-8" }).trim();
  } catch {}
  // Common locations on macOS / Linux
  for (const candidate of ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg", "/usr/bin/ffmpeg"]) {
    try {
      if (fs.existsSync(candidate)) return candidate;
    } catch {}
  }
  return null;
}

const FFMPEG_PATH = findFfmpeg();
const MAX_DURATION_SEC = 30;

// VAD (Voice Activity Detection) settings
const SILENCE_NOISE = "-25dB";    // amplitude threshold for silence (must be above mic noise floor)
const SILENCE_DURATION = "1.5";   // seconds of continuous silence to trigger
const GRACE_PERIOD_MS = 8000;     // max wait for first speech before giving up
const MIN_SPEECH_SEC = 1.0;       // minimum above-threshold audio to count as speech

const RUNTIME_DIR_ENV_KEYS = ["HIMD_VOICE_BRIDGE_DIR"];

/**
 * Get the runtime directory for temp audio and control files.
 * Defaults to the OS temp directory so installed packages do not write next to
 * their install location.
 */
function getBaseDir() {
  for (const envKey of RUNTIME_DIR_ENV_KEYS) {
    const configuredValue = process.env[envKey];
    if (configuredValue && configuredValue.trim()) {
      return path.resolve(configuredValue);
    }
  }

  return path.join(os.tmpdir(), "himd-voice-bridge");
}

/**
 * Get the control directory for PID/signal files.
 */
function getControlDir() {
  const dir = path.join(getBaseDir(), "control");
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}

/**
 * Generate output path: {runtime_dir}/captures/{YYYY-MM-DD}/{epoch_ms}.wav
 */
function generateOutputPath(baseDir) {
  const now = new Date();
  const dateStr = now.toISOString().slice(0, 10);
  const dir = path.join(baseDir, "captures", dateStr);
  fs.mkdirSync(dir, { recursive: true });
  return path.join(dir, `${Date.now()}.wav`);
}

/**
 * Parse duration from WAV file header.
 * Returns duration in milliseconds.
 */
function getWavDurationMs(filePath) {
  try {
    const buf = fs.readFileSync(filePath);
    if (buf.length < 44 || buf.toString("ascii", 0, 4) !== "RIFF") return 0;
    const sampleRate = buf.readUInt32LE(24);
    const channels = buf.readUInt16LE(22);
    const bitsPerSample = buf.readUInt16LE(34);
    let offset = 12;
    while (offset < buf.length - 8) {
      const chunkId = buf.toString("ascii", offset, offset + 4);
      const chunkSize = buf.readUInt32LE(offset + 4);
      if (chunkId === "data") {
        const totalSamples = chunkSize / (bitsPerSample / 8);
        return Math.round((totalSamples / (sampleRate * channels)) * 1000);
      }
      offset += 8 + chunkSize;
    }
    return 0;
  } catch {
    return 0;
  }
}

/**
 * Capture audio from default microphone.
 *
 * Stops automatically when:
 *   1. User finishes speaking (silence detected after speech) → stopped_by: "silence"
 *   2. No speech detected within grace period              → stopped_by: "no_speech"
 *   3. External stop signal file created                   → stopped_by: "manual"
 *   4. Max duration reached                                → stopped_by: "timeout"
 *
 * @param {number} maxDurationSec - Safety cap in seconds (default 30, max 60)
 * @returns {Promise<object>} Capture result
 */
function captureOnce(maxDurationSec = 30) {
  return new Promise((resolve, reject) => {
    if (!FFMPEG_PATH) {
      try {
        assertCommandAvailable("ffmpeg", "brew install ffmpeg");
      } catch (err) {
        reject(err);
        return;
      }
    }

    const cappedDuration = Math.max(1, Math.min(maxDurationSec, 60));
    const outputPath = generateOutputPath(getBaseDir());
    const controlDir = getControlDir();

    const args = [
      "-f", "avfoundation",
      "-i", ":0",
      "-af", `silencedetect=noise=${SILENCE_NOISE}:d=${SILENCE_DURATION}`,
      "-t", String(cappedDuration),
      "-ar", "16000",
      "-ac", "1",
      "-y",
      outputPath,
    ];

    const ffmpeg = spawn(FFMPEG_PATH, args, {
      stdio: ["pipe", "pipe", "pipe"],
    });

    let stderrOutput = "";
    let stoppedBy = "timeout";
    let speechDetected = false;
    let stopping = false;

    // Write PID file so external tools can find the process
    const pidFile = path.join(controlDir, ".capture-pid");
    fs.writeFileSync(pidFile, String(ffmpeg.pid));

    function stopRecording(reason) {
      if (stopping) return;
      stopping = true;
      stoppedBy = reason;
      try { ffmpeg.kill("SIGTERM"); } catch {}
    }

    // Grace period: if no speech within GRACE_PERIOD_MS, stop
    const graceTimer = setTimeout(() => {
      if (!speechDetected) {
        stopRecording("no_speech");
      }
    }, GRACE_PERIOD_MS);

    // Parse ffmpeg stderr for silencedetect events
    ffmpeg.stderr.on("data", (data) => {
      const chunk = data.toString();
      stderrOutput += chunk;

      // "silence_end" means a silence period just ended → speech started
      // This handles Scenario A: recording starts with silence, then user speaks
      if (chunk.includes("silence_end")) {
        speechDetected = true;
        clearTimeout(graceTimer);
      }

      // "silence_start" means audio dropped below threshold → possible end of speech
      if (chunk.includes("silence_start")) {
        if (speechDetected) {
          // Speech was already detected via silence_end, now it ended → stop
          setTimeout(() => stopRecording("silence"), 300);
        } else {
          // Scenario B: recording started with above-threshold audio (no initial
          // silence detected). If silence_start timestamp is large enough, the
          // audio before it was speech, not just a brief noise spike.
          const match = chunk.match(/silence_start:\s*([\d.]+)/);
          if (match && parseFloat(match[1]) > MIN_SPEECH_SEC) {
            speechDetected = true;
            clearTimeout(graceTimer);
            setTimeout(() => stopRecording("silence"), 300);
          }
        }
      }
    });

    // Poll for external stop signal file
    const stopFile = path.join(controlDir, ".stop-capture");
    const pollInterval = setInterval(() => {
      if (fs.existsSync(stopFile)) {
        try { fs.unlinkSync(stopFile); } catch {}
        clearInterval(pollInterval);
        stopRecording("manual");
      }
    }, 200);

    ffmpeg.on("close", () => {
      clearTimeout(graceTimer);
      clearInterval(pollInterval);
      try { fs.unlinkSync(pidFile); } catch {}

      if (!fs.existsSync(outputPath)) {
        reject(new Error(`Recording failed: output file not created.\nffmpeg stderr: ${stderrOutput.slice(-500)}`));
        return;
      }

      const stat = fs.statSync(outputPath);
      const durationMs = getWavDurationMs(outputPath);

      resolve({
        temp_audio_path: outputPath,
        format: "wav",
        duration_ms: durationMs,
        sample_rate: 16000,
        channels: 1,
        file_size_bytes: stat.size,
        stopped_by: stoppedBy,
      });
    });

    ffmpeg.on("error", (err) => {
      clearTimeout(graceTimer);
      clearInterval(pollInterval);
      try { fs.unlinkSync(pidFile); } catch {}
      reject(new Error(`Failed to start ffmpeg: ${err.message}`));
    });
  });
}

module.exports = {
  captureOnce,
  getWavDurationMs,
  generateOutputPath,
  getBaseDir,
  getControlDir,
};
