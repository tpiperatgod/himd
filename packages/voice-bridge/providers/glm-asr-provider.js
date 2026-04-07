/**
 * GLM-ASR provider — extracted from original index.js.
 *
 * Returns transcript-only results (no enriched understanding).
 */

const fs = require("fs");
const path = require("path");
const { createEmptyResult } = require("./audio-provider");

const ZHIPU_API_KEY = process.env.ZHIPU_API_KEY;
const ASR_API_URL = "https://open.bigmodel.cn/api/paas/v4/audio/transcriptions";

const name = "glm-asr";

/**
 * Transcribe audio file via GLM-ASR-2512.
 * @param {string} filePath - Absolute path to audio file
 * @returns {Promise<object>} AudioUnderstandingResult with transcript only
 */
async function understand(filePath) {
  if (!ZHIPU_API_KEY) {
    throw new Error("ZHIPU_API_KEY environment variable is not set");
  }

  const fileBuffer = fs.readFileSync(filePath);
  const fileName = path.basename(filePath);

  const formData = new FormData();
  formData.append("model", "glm-asr-2512");
  formData.append("file", new Blob([fileBuffer]), fileName);

  const response = await fetch(ASR_API_URL, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${ZHIPU_API_KEY}`,
    },
    body: formData,
  });

  const result = await response.json();

  if (result.error) {
    throw new Error(`ASR API error: ${result.error.message || JSON.stringify(result.error)}`);
  }

  const transcript = result.text || "";
  const model = result.model || "glm-asr-2512";

  const output = createEmptyResult(name, model);
  output.transcript = transcript;
  output.confidence = 0.8;

  return output;
}

module.exports = { name, understand };
