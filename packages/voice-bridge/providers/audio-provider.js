/**
 * Audio provider factory.
 *
 * Every provider MUST export:
 *   async understand(filePath) → AudioUnderstandingResult
 *   readonly name: string
 *
 * Provider selection reads HIMD_AUDIO_PROVIDER (defaults to "qwen-omni").
 */

/**
 * Create an empty result with default values.
 * @param {string} provider
 * @param {string} model
 * @returns {object}
 */
function createEmptyResult(provider, model) {
  return {
    transcript: "",
    provider,
    model,
    summary: null,
    intent: null,
    emotion: null,
    tone: null,
    key_points: null,
    non_verbal_signals: null,
    language: null,
    confidence: 0.0,
    raw_text: null,
    warnings: [],
  };
}

/**
 * Get the configured audio provider.
 * Reads HIMD_AUDIO_PROVIDER (defaults to "qwen-omni").
 * Falls back through: primary → HIMD_FALLBACK_PROVIDER → none
 */
function getAudioProvider() {
  const name = (process.env.HIMD_AUDIO_PROVIDER || "qwen-omni").toLowerCase();

  switch (name) {
    case "qwen-omni":
      return require("./qwen-omni-provider");
    case "glm-asr":
      return require("./glm-asr-provider");
    default:
      throw new Error(`Unknown audio provider: ${name}. Use "qwen-omni" or "glm-asr".`);
  }
}

/**
 * Get the fallback provider (if configured).
 * @returns {object|null} Provider module or null if disabled
 */
function getFallbackProvider() {
  const fallback = (process.env.HIMD_FALLBACK_PROVIDER || "glm-asr").toLowerCase();
  if (fallback === "none") return null;
  if (fallback === "glm-asr") return require("./glm-asr-provider");
  if (fallback === "qwen-omni") return require("./qwen-omni-provider");
  return null;
}

module.exports = { getAudioProvider, getFallbackProvider, createEmptyResult };
