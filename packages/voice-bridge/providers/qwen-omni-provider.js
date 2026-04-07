/**
 * Qwen3.5-Omni audio understanding provider.
 *
 * Uses DashScope's OpenAI-compatible endpoint to send audio + text,
 * receiving structured JSON with transcript, emotion, tone, etc.
 *
 * Key constraints from Qwen-Omni docs:
 * - stream: true is REQUIRED
 * - Single user message can contain text + one modality (input_audio)
 * - input_audio.data accepts URLs or base64 data URLs
 * - modalities: ["text"] for text-only output
 */

const fs = require("fs");
const path = require("path");
const { createEmptyResult } = require("./audio-provider");
const { fileToBase64 } = require("../audio-utils");
const { parseJsonResponse } = require("../audio-utils");
const { SYSTEM_PROMPT, USER_PROMPT } = require("../prompts/qwen-omni-prompt");

const name = "qwen-omni";

const DASHSCOPE_API_KEY = process.env.DASHSCOPE_API_KEY;
const DASHSCOPE_BASE_URL =
  process.env.DASHSCOPE_BASE_URL ||
  "https://dashscope.aliyuncs.com/compatible-mode/v1";
const QWEN_OMNI_MODEL = process.env.QWEN_OMNI_MODEL || "qwen3-omni-flash";
const QWEN_OMNI_TIMEOUT_MS = parseInt(process.env.QWEN_OMNI_TIMEOUT_MS || "30000", 10);
const QWEN_OMNI_DEBUG = process.env.QWEN_OMNI_DEBUG === "true";

/**
 * Debug logger — only active when QWEN_OMNI_DEBUG=true.
 */
function debug(...args) {
  if (QWEN_OMNI_DEBUG) {
    const timestamp = new Date().toISOString().slice(11, 19);
    console.error(`[qwen-omni ${timestamp}]`, ...args);
  }
}

/**
 * Classify an HTTP error for retry/fallback decisions.
 * @param {number} status
 * @param {string} body
 * @returns {{ retryable: boolean, action: string }}
 */
function classifyHttpError(status, body) {
  if (status === 429) return { retryable: false, action: "rate_limited" };
  if (status >= 500) return { retryable: true, action: "server_error" };
  if (status === 401 || status === 403) return { retryable: false, action: "auth_error" };
  return { retryable: false, action: `http_${status}` };
}

/**
 * Make a single request to Qwen Omni API and collect streamed text.
 * @param {string} audioDataUrl - base64 data URL for the audio
 * @param {string} audioFormat - file extension (wav, mp3, etc.)
 * @param {number} timeoutMs - request timeout
 * @returns {Promise<string>} Full text content from streaming response
 */
async function makeRequest(audioDataUrl, audioFormat, timeoutMs) {
  const url = `${DASHSCOPE_BASE_URL}/chat/completions`;

  const body = {
    model: QWEN_OMNI_MODEL,
    stream: true,
    stream_options: { include_usage: true },
    modalities: ["text"],
    messages: [
      { role: "system", content: SYSTEM_PROMPT },
      {
        role: "user",
        content: [
          {
            type: "input_audio",
            input_audio: {
              data: audioDataUrl,
              format: audioFormat,
            },
          },
          { type: "text", text: USER_PROMPT },
        ],
      },
    ],
  };

  debug(`POST ${url} model=${QWEN_OMNI_MODEL} format=${audioFormat}`);

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);

  const startTime = Date.now();

  try {
    const response = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${DASHSCOPE_API_KEY}`,
      },
      body: JSON.stringify(body),
      signal: controller.signal,
    });

    if (!response.ok) {
      const errBody = await response.text().catch(() => "");
      const classified = classifyHttpError(response.status, errBody);
      const err = new Error(`Qwen Omni API error (${response.status}): ${errBody.slice(0, 200)}`);
      err.status = response.status;
      err.classified = classified;
      throw err;
    }

    // Collect streamed text chunks using a line buffer to handle
    // SSE data lines that may be split across network chunk boundaries.
    let fullText = "";
    let lineBuffer = "";
    const reader = response.body.getReader();
    const decoder = new TextDecoder();

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      lineBuffer += decoder.decode(value, { stream: true });

      // Process complete lines only (delimited by \n)
      let newlineIdx;
      while ((newlineIdx = lineBuffer.indexOf("\n")) !== -1) {
        const line = lineBuffer.slice(0, newlineIdx).trim();
        lineBuffer = lineBuffer.slice(newlineIdx + 1);

        if (!line.startsWith("data: ")) continue;
        const data = line.slice(6).trim();
        if (data === "[DONE]") continue;

        try {
          const parsed = JSON.parse(data);
          if (parsed.choices && parsed.choices[0]?.delta?.content) {
            fullText += parsed.choices[0].delta.content;
          }
        } catch {
          // Skip unparseable SSE lines
        }
      }
    }

    // Process any remaining content in the buffer (last line without trailing \n)
    const remaining = lineBuffer.trim();
    if (remaining.startsWith("data: ")) {
      const data = remaining.slice(6).trim();
      if (data !== "[DONE]") {
        try {
          const parsed = JSON.parse(data);
          if (parsed.choices && parsed.choices[0]?.delta?.content) {
            fullText += parsed.choices[0].delta.content;
          }
        } catch {
          // Skip unparseable SSE lines
        }
      }
    }

    const elapsed = Date.now() - startTime;
    debug(`Response collected in ${elapsed}ms, ${fullText.length} chars`);

    return fullText;
  } finally {
    clearTimeout(timeout);
  }
}

/**
 * Normalize parsed JSON into the standard result schema.
 * @param {object} parsed - Raw parsed JSON from model
 * @param {string} rawText - Original text for debugging
 * @returns {object} Normalized AudioUnderstandingResult
 */
function normalizeResult(parsed, rawText) {
  const result = createEmptyResult(name, QWEN_OMNI_MODEL);

  // Required
  result.transcript = typeof parsed.transcript === "string" ? parsed.transcript : "";
  result.raw_text = rawText;

  // Optional enriched fields
  result.summary = typeof parsed.summary === "string" ? parsed.summary : null;
  result.intent = typeof parsed.intent === "string" ? parsed.intent : null;

  // Emotion: { primary, confidence } or string
  if (parsed.emotion) {
    if (typeof parsed.emotion === "object") {
      result.emotion = {
        primary: parsed.emotion.primary || "unknown",
        confidence: typeof parsed.emotion.confidence === "number" ? parsed.emotion.confidence : null,
      };
    } else if (typeof parsed.emotion === "string") {
      result.emotion = { primary: parsed.emotion, confidence: null };
    }
  }

  // Tone: string[]
  result.tone = Array.isArray(parsed.tone) ? parsed.tone : null;

  // Key points: string[]
  result.key_points = Array.isArray(parsed.key_points) ? parsed.key_points : null;

  // Non-verbal signals: string[]
  result.non_verbal_signals = Array.isArray(parsed.non_verbal_signals) ? parsed.non_verbal_signals : null;

  // Language
  result.language = typeof parsed.language === "string" ? parsed.language : null;

  // Overall confidence
  result.confidence = typeof parsed.confidence === "number" ? parsed.confidence : 0.5;

  return result;
}

/**
 * Main entry: understand audio file via Qwen Omni.
 * @param {string} filePath - Absolute path to audio file
 * @returns {Promise<object>} AudioUnderstandingResult
 */
async function understand(filePath) {
  if (!DASHSCOPE_API_KEY) {
    throw new Error("DASHSCOPE_API_KEY environment variable is not set. Set it or switch HIMD_AUDIO_PROVIDER to glm-asr.");
  }

  if (!fs.existsSync(filePath)) {
    throw new Error(`Audio file not found: ${filePath}`);
  }

  // Read and encode audio
  const { base64, ext } = fileToBase64(filePath);
  const audioDataUrl = `data:audio/${ext};base64,${base64}`;

  debug(`File: ${path.basename(filePath)} (${ext}, ${Math.round(base64.length * 0.75 / 1024)}KB)`);

  // Attempt request with retry on transient errors
  let rawText;
  let lastError;

  for (let attempt = 0; attempt < 2; attempt++) {
    try {
      rawText = await makeRequest(audioDataUrl, ext, QWEN_OMNI_TIMEOUT_MS);
      break;
    } catch (err) {
      lastError = err;
      if (err.classified && err.classified.retryable && attempt === 0) {
        debug(`Retry after: ${err.message.slice(0, 100)}`);
        continue;
      }
      throw err;
    }
  }

  if (rawText === undefined) throw lastError;

  // Empty response
  if (!rawText || rawText.trim().length === 0) {
    const result = createEmptyResult(name, QWEN_OMNI_MODEL);
    result.warnings.push("empty_response");
    return result;
  }

  // Parse JSON response
  const parsed = parseJsonResponse(rawText);

  if (parsed) {
    return normalizeResult(parsed, rawText);
  }

  // JSON parse failed — use raw text as transcript
  debug(`JSON parse failed, using raw text as transcript`);
  const result = createEmptyResult(name, QWEN_OMNI_MODEL);
  result.transcript = rawText.trim();
  result.raw_text = rawText;
  result.warnings.push("json_parse_failed");
  return result;
}

module.exports = { name, understand };
