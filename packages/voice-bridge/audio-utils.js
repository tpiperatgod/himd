const fs = require("fs");
const path = require("path");

/**
 * Read a local audio file and return its base64-encoded content.
 * @param {string} filePath - Absolute path to the audio file
 * @returns {{ base64: string, ext: string, mimeType: string }}
 */
function fileToBase64(filePath) {
  const ext = path.extname(filePath).toLowerCase().replace(".", "");
  const buffer = fs.readFileSync(filePath);
  const base64 = buffer.toString("base64");

  const mimeMap = {
    wav: "audio/wav",
    mp3: "audio/mpeg",
    aac: "audio/aac",
    ogg: "audio/ogg",
    flac: "audio/flac",
    m4a: "audio/mp4",
    amr: "audio/amr",
    "3gp": "audio/3gpp",
  };

  const mimeType = mimeMap[ext] || "audio/wav";

  return { base64, ext, mimeType };
}

/**
 * Build a data URL suitable for Qwen Omni input_audio.
 * Format: data:audio/{ext};base64,{base64Audio}
 */
function buildAudioDataUrl(filePath) {
  const { base64, ext } = fileToBase64(filePath);
  return `data:audio/${ext};base64,${base64}`;
}

/**
 * Attempt to parse a JSON string with progressive fallback.
 * 1. Direct JSON.parse
 * 2. Extract JSON block with regex
 * 3. Repair common issues (truncated, missing brackets)
 * 4. Return null if all fail
 */
function parseJsonResponse(text) {
  if (!text || typeof text !== "string") return null;

  // Strip markdown fences if present
  let cleaned = text.trim();
  if (cleaned.startsWith("```")) {
    cleaned = cleaned.replace(/^```(?:json)?\s*\n?/, "").replace(/\n?```\s*$/, "");
  }

  // 1. Direct parse
  try {
    return JSON.parse(cleaned);
  } catch {}

  // 2. Extract JSON object from surrounding text
  try {
    const match = cleaned.match(/\{[\s\S]*\}/);
    if (match) return JSON.parse(match[0]);
  } catch {}

  // 3. Repair: try to fix truncated/missing brackets
  try {
    let repaired = cleaned;
    // Count brackets
    const opens = (repaired.match(/\{/g) || []).length;
    const closes = (repaired.match(/\}/g) || []).length;
    if (opens > closes) {
      repaired += "}".repeat(opens - closes);
    }
    // Fix truncated strings at end
    repaired = repaired.replace(/"[^"]*$/, '""');
    // Fix trailing commas before } or ]
    repaired = repaired.replace(/,\s*([\]}])/g, "$1");
    return JSON.parse(repaired);
  } catch {}

  return null;
}

module.exports = { fileToBase64, buildAudioDataUrl, parseJsonResponse };
