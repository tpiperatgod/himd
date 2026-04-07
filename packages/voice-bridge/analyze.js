const fs = require("fs");

/**
 * Analyze audio file for basic acoustic features.
 * All values are engineering approximations, not ML-inferred.
 *
 * Stable fields: speech_rate, energy, pause_pattern
 * Unstable fields: emotion (defaults to "neutral")
 */

// Rough thresholds for Chinese speech (~3 chars/sec = normal)
const SPEECH_RATE_SLOW_THRESHOLD = 2.0;   // chars/sec below this = slow
const SPEECH_RATE_FAST_THRESHOLD = 5.0;   // chars/sec above this = fast

// RMS energy thresholds (16-bit PCM, normalized)
// Voice recordings typically range 0.02-0.30 RMS
const ENERGY_LOW_THRESHOLD = 0.08;
const ENERGY_HIGH_THRESHOLD = 0.30;

/**
 * Compute RMS energy from raw PCM samples.
 * Works with 16-bit little-endian mono PCM.
 */
function computeRmsEnergy(buffer) {
  let sum = 0;
  const sampleCount = Math.floor(buffer.length / 2); // 16-bit samples
  for (let i = 0; i < sampleCount; i++) {
    const sample = buffer.readInt16LE(i * 2);
    const normalized = sample / 32768.0;
    sum += normalized * normalized;
  }
  return Math.sqrt(sum / sampleCount);
}

/**
 * Estimate speech rate from transcript length and audio duration.
 * Uses character count as proxy for word count (Chinese).
 */
function estimateSpeechRate(transcript, durationSec) {
  if (!transcript || durationSec <= 0) return { value: "normal", confidence: 0.3 };

  const charCount = transcript.replace(/\s/g, "").length;
  const rate = charCount / durationSec;

  let value;
  if (rate < SPEECH_RATE_SLOW_THRESHOLD) {
    value = "slow";
  } else if (rate > SPEECH_RATE_FAST_THRESHOLD) {
    value = "fast";
  } else {
    value = "normal";
  }

  return { value, confidence: 0.7, rate_chars_per_sec: Math.round(rate * 10) / 10 };
}

/**
 * Classify energy level from RMS value.
 */
function classifyEnergy(rms) {
  if (rms < ENERGY_LOW_THRESHOLD) return { value: "low", confidence: 0.6 };
  if (rms > ENERGY_HIGH_THRESHOLD) return { value: "high", confidence: 0.6 };
  return { value: "medium", confidence: 0.6 };
}

/**
 * Estimate pause pattern from audio duration vs expected speech duration.
 * Chinese speech ~3 chars/sec. If audio is much longer than expected speech,
 * there are likely more/longer pauses.
 */
function estimatePausePattern(transcript, durationSec) {
  if (!transcript || durationSec <= 0) return { value: "medium", confidence: 0.4 };

  const charCount = transcript.replace(/\s/g, "").length;
  const expectedSpeechDuration = charCount / 3.0; // ~3 chars/sec baseline
  const pauseRatio = durationSec / Math.max(expectedSpeechDuration, 0.5);

  let value;
  if (pauseRatio < 1.3) {
    value = "short";  // few pauses, compact speech
  } else if (pauseRatio > 2.0) {
    value = "long";   // many/long pauses
  } else {
    value = "medium";
  }

  return { value, confidence: 0.5, pause_ratio: Math.round(pauseRatio * 10) / 10 };
}

/**
 * Main analysis function.
 * @param {string} filePath - Path to audio file
 * @param {string} transcript - ASR transcript text
 * @returns {object} Analysis result with all fields
 */
function analyzeAudio(filePath, transcript) {
  // Read raw file for energy analysis
  let rmsEnergy = 0;
  let durationSec = 0;

  try {
    const fileBuffer = fs.readFileSync(filePath);

    // Try to parse as WAV to get PCM data
    // WAV header: bytes 16-19 = subchunk1Size, bytes 20-21 = audioFormat,
    // bytes 22-23 = numChannels, bytes 24-27 = sampleRate,
    // bytes 34-35 = bitsPerSample, data starts at byte 44 (typically)
    if (fileBuffer.length > 44 &&
      fileBuffer.toString("ascii", 0, 4) === "RIFF" &&
      fileBuffer.toString("ascii", 8, 12) === "WAVE") {

    const numChannels = fileBuffer.readUInt16LE(22);
    const sampleRate = fileBuffer.readUInt32LE(24);
    const bitsPerSample = fileBuffer.readUInt16LE(34);

    // Find "data" chunk
    let dataOffset = 12;
    while (dataOffset < fileBuffer.length - 8) {
      const chunkId = fileBuffer.toString("ascii", dataOffset, dataOffset + 4);
      const chunkSize = fileBuffer.readUInt32LE(dataOffset + 4);
      if (chunkId === "data") {
        dataOffset += 8;
        const pcmData = fileBuffer.subarray(dataOffset, dataOffset + chunkSize);
        rmsEnergy = computeRmsEnergy(pcmData);
        const totalSamples = chunkSize / (bitsPerSample / 8);
        durationSec = totalSamples / (sampleRate * numChannels);
        break;
      }
      dataOffset += 8 + chunkSize;
    }
    } else {
      // Not a WAV or can't parse - use file size as rough proxy
      // Assume MP3 ~128kbps = 16KB/sec
      durationSec = fileBuffer.length / 16000;
      rmsEnergy = 0.1; // default medium
    }
  } catch (err) {
    // If we can't read the file for analysis, use defaults
    return {
      speech_rate: { value: "normal", confidence: 0.1 },
      energy: { value: "medium", confidence: 0.1 },
      pause_pattern: { value: "medium", confidence: 0.1 },
      overall_confidence: 0.2,
      _note: "Audio file could not be read for analysis, using defaults",
    };
  }

  const speechRate = estimateSpeechRate(transcript, durationSec);
  const energy = classifyEnergy(rmsEnergy);
  const pausePattern = estimatePausePattern(transcript, durationSec);

  // Overall confidence: average of individual confidences
  const confidences = [speechRate.confidence, energy.confidence, pausePattern.confidence];
  const overallConfidence = confidences.reduce((a, b) => a + b, 0) / confidences.length;

  return {
    speech_rate: speechRate,
    energy,
    pause_pattern: pausePattern,
    overall_confidence: Math.round(overallConfidence * 100) / 100,
    _duration_sec: Math.round(durationSec * 10) / 10,
    _rms_energy: Math.round(rmsEnergy * 1000) / 1000,
  };
}

/**
 * Format analysis into clean audio_turn structure.
 * Now accepts enriched provider result and merges audio_understanding.
 *
 * @param {object} providerResult - AudioUnderstandingResult from provider
 * @param {object} rawAnalysis - Local acoustic analysis
 * @param {string} filePath - Path to audio file
 * @returns {object} audio_turn structure
 */
function buildAudioTurn(providerResult, rawAnalysis, filePath) {
  const analysis = {
    speech_rate: rawAnalysis.speech_rate.value,
    energy: rawAnalysis.energy.value,
    pause_pattern: rawAnalysis.pause_pattern.value,
  };

  const turn = {
    transcript: providerResult.transcript,
    analysis,
    analysis_confidence: rawAnalysis.overall_confidence,
    source: "file",
    audio_file: filePath,
    model: providerResult.model,
    provider: providerResult.provider,
  };

  // Add enriched audio understanding if provider returned non-trivial fields
  const hasEnrichment =
    providerResult.summary ||
    providerResult.intent ||
    providerResult.emotion ||
    providerResult.tone ||
    providerResult.key_points ||
    providerResult.non_verbal_signals ||
    providerResult.language;

  if (hasEnrichment) {
    turn.audio_understanding = {};

    if (providerResult.summary) turn.audio_understanding.summary = providerResult.summary;
    if (providerResult.intent) turn.audio_understanding.intent = providerResult.intent;
    if (providerResult.emotion) turn.audio_understanding.emotion = providerResult.emotion;
    if (providerResult.tone) turn.audio_understanding.tone = providerResult.tone;
    if (providerResult.key_points) turn.audio_understanding.key_points = providerResult.key_points;
    if (providerResult.non_verbal_signals) turn.audio_understanding.non_verbal_signals = providerResult.non_verbal_signals;
    if (providerResult.language) turn.audio_understanding.language = providerResult.language;

    turn.audio_understanding.confidence = providerResult.confidence;
  }

  // Pass through warnings if any
  if (providerResult.warnings && providerResult.warnings.length > 0) {
    turn.warnings = providerResult.warnings;
  }

  return turn;
}

module.exports = { analyzeAudio, buildAudioTurn };
