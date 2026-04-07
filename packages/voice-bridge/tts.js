const fs = require("fs");
const { execSync } = require("child_process");
const { assertCommandAvailable } = require("./system-deps.js");

const ZHIPU_API_KEY = process.env.ZHIPU_API_KEY;
const TTS_API_URL = "https://open.bigmodel.cn/api/paas/v4/audio/speech";
const PROFILE_PATH = "/tmp/himd-voice-profile.json";
const MARKER_PATH = "/tmp/himd-last-speech-turn";

const VALID_VOICES = ["tongtong", "chuichui", "xiaochen", "jam", "kazi", "douji", "luodo"];
const DEFAULT_PROFILE = { voice: "tongtong", speed: 1.0 };

function readProfile() {
  try {
    const data = fs.readFileSync(PROFILE_PATH, "utf-8");
    return { ...DEFAULT_PROFILE, ...JSON.parse(data) };
  } catch {
    return { ...DEFAULT_PROFILE };
  }
}

function writeProfile(updates) {
  const current = readProfile();
  if (updates.voice !== undefined) {
    if (!VALID_VOICES.includes(updates.voice)) {
      throw new Error(`Invalid voice: ${updates.voice}. Valid: ${VALID_VOICES.join(", ")}`);
    }
    current.voice = updates.voice;
  }
  if (updates.speed !== undefined) {
    if (updates.speed < 0.5 || updates.speed > 2.0) {
      throw new Error(`Speed must be between 0.5 and 2.0, got ${updates.speed}`);
    }
    current.speed = updates.speed;
  }
  current.updated_at = new Date().toISOString();
  fs.writeFileSync(PROFILE_PATH, JSON.stringify(current, null, 2));
  return current;
}

async function synthesize(text, options = {}) {
  if (!ZHIPU_API_KEY) {
    throw new Error("ZHIPU_API_KEY environment variable is not set");
  }
  if (!text || text.trim().length === 0) {
    throw new Error("Text is required for TTS");
  }
  if (text.length > 1024) {
    throw new Error(`Text too long: ${text.length} chars (max 1024)`);
  }

  const profile = readProfile();
  const voice = options.voice || profile.voice;
  const speed = options.speed !== undefined ? options.speed : profile.speed;

  const response = await fetch(TTS_API_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${ZHIPU_API_KEY}`,
    },
    body: JSON.stringify({
      model: "glm-tts",
      input: text,
      voice,
      response_format: "wav",
      speed,
    }),
  });

  if (!response.ok) {
    const errBody = await response.text();
    throw new Error(`TTS API error (${response.status}): ${errBody}`);
  }

  const timestamp = Date.now();
  const audioFile = buildAudioFilePath(timestamp);

  const arrayBuffer = await response.arrayBuffer();
  fs.writeFileSync(audioFile, Buffer.from(arrayBuffer));

  return { audioFile, voice, speed, textLength: text.length };
}

function playAudio(audioFile) {
  assertCommandAvailable("afplay", "macOS includes afplay by default; confirm Command Line Tools and audio playback support with: xcode-select --install");
  try {
    execSync(`afplay "${audioFile}"`, { timeout: 30000 });
    return true;
  } catch (err) {
    return false;
  }
}

function markSpeechTurn() {
  fs.writeFileSync(MARKER_PATH, Date.now().toString());
}

function checkRecentSpeech(maxAgeMs = 60000) {
  try {
    const ts = parseInt(fs.readFileSync(MARKER_PATH, "utf-8"), 10);
    return Date.now() - ts < maxAgeMs;
  } catch {
    return false;
  }
}

function buildAudioFilePath(timestamp = Date.now()) {
  return `/tmp/himd-tts-${timestamp}.wav`;
}

module.exports = {
  synthesize,
  playAudio,
  markSpeechTurn,
  checkRecentSpeech,
  readProfile,
  writeProfile,
  VALID_VOICES,
  TTS_API_URL,
  PROFILE_PATH,
  MARKER_PATH,
  buildAudioFilePath,
};
