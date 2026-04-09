//! Local acoustic analysis — reads WAV PCM data directly (no external deps).
//!
//! Computes speech_rate, energy, and pause_pattern from raw audio + transcript.
//! Ported from the Node.js `analyze.js` module.

use crate::errors::HimdError;
use crate::provider::AudioUnderstandingResult;
use crate::types::{
    AcousticAnalysis, AudioTurn, AudioUnderstanding, Emotion, EnergyLevel, PausePattern, SpeechRate,
};

// ---------------------------------------------------------------------------
// Thresholds (matching Node.js analyze.js exactly)
// ---------------------------------------------------------------------------

/// Chars/sec below this = slow speech.
const SPEECH_RATE_SLOW_THRESHOLD: f64 = 2.0;
/// Chars/sec above this = fast speech.
const SPEECH_RATE_FAST_THRESHOLD: f64 = 5.0;

/// RMS energy below this = low.
const ENERGY_LOW_THRESHOLD: f64 = 0.08;
/// RMS energy above this = high.
const ENERGY_HIGH_THRESHOLD: f64 = 0.30;

/// Pause ratio below this = short pauses.
const PAUSE_RATIO_SHORT_THRESHOLD: f64 = 1.3;
/// Pause ratio above this = long pauses.
const PAUSE_RATIO_LONG_THRESHOLD: f64 = 2.0;

// ---------------------------------------------------------------------------
// WAV parsing
// ---------------------------------------------------------------------------

/// Parsed WAV header info.
struct WavInfo {
    num_channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    data_offset: usize,
    data_size: usize,
}

/// Parse a WAV file, returning header info or None if not a valid WAV.
fn parse_wav_header(data: &[u8]) -> Option<WavInfo> {
    if data.len() < 44 {
        return None;
    }
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }

    let num_channels = u16::from_le_bytes([data[22], data[23]]);
    let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let bits_per_sample = u16::from_le_bytes([data[34], data[35]]);

    // Find "data" chunk
    let mut offset = 12;
    while offset < data.len().saturating_sub(8) {
        let chunk_id = &data[offset..offset + 4];
        let chunk_size = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        if chunk_id == b"data" {
            return Some(WavInfo {
                num_channels,
                sample_rate,
                bits_per_sample,
                data_offset: offset + 8,
                data_size: chunk_size,
            });
        }
        offset += 8 + chunk_size;
    }

    None
}

// ---------------------------------------------------------------------------
// Acoustic measurements
// ---------------------------------------------------------------------------

/// Compute RMS energy from 16-bit little-endian PCM samples.
fn compute_rms_energy(pcm_data: &[u8]) -> f64 {
    let sample_count = pcm_data.len() / 2;
    if sample_count == 0 {
        return 0.0;
    }

    let mut sum = 0.0f64;
    for i in 0..sample_count {
        let sample = i16::from_le_bytes([pcm_data[i * 2], pcm_data[i * 2 + 1]]);
        let normalized = sample as f64 / 32768.0;
        sum += normalized * normalized;
    }
    (sum / sample_count as f64).sqrt()
}

/// Classify energy level from RMS value.
fn classify_energy(rms: f64) -> EnergyLevel {
    if rms < ENERGY_LOW_THRESHOLD {
        EnergyLevel::Low
    } else if rms > ENERGY_HIGH_THRESHOLD {
        EnergyLevel::High
    } else {
        EnergyLevel::Medium
    }
}

/// Estimate speech rate from transcript length and audio duration.
fn estimate_speech_rate(transcript: &str, duration_sec: f64) -> SpeechRate {
    if transcript.is_empty() || duration_sec <= 0.0 {
        return SpeechRate::Normal;
    }

    let char_count = transcript.chars().filter(|c| !c.is_whitespace()).count() as f64;
    let rate = char_count / duration_sec;

    if rate < SPEECH_RATE_SLOW_THRESHOLD {
        SpeechRate::Slow
    } else if rate > SPEECH_RATE_FAST_THRESHOLD {
        SpeechRate::Fast
    } else {
        SpeechRate::Normal
    }
}

/// Estimate pause pattern from audio duration vs expected speech duration.
fn estimate_pause_pattern(transcript: &str, duration_sec: f64) -> PausePattern {
    if transcript.is_empty() || duration_sec <= 0.0 {
        return PausePattern::Medium;
    }

    let char_count = transcript.chars().filter(|c| !c.is_whitespace()).count() as f64;
    let expected_speech_duration = char_count / 3.0; // ~3 chars/sec baseline for Chinese
    let pause_ratio = duration_sec / expected_speech_duration.max(0.5);

    if pause_ratio < PAUSE_RATIO_SHORT_THRESHOLD {
        PausePattern::Short
    } else if pause_ratio > PAUSE_RATIO_LONG_THRESHOLD {
        PausePattern::Long
    } else {
        PausePattern::Medium
    }
}

// ---------------------------------------------------------------------------
// Analysis result (internal)
// ---------------------------------------------------------------------------

/// Internal acoustic analysis result.
struct RawAnalysis {
    speech_rate: SpeechRate,
    energy: EnergyLevel,
    pause_pattern: PausePattern,
    overall_confidence: f64,
}

/// Run local acoustic analysis on a WAV file.
fn analyze_audio(file_path: &str, transcript: &str) -> Result<RawAnalysis, HimdError> {
    let data = std::fs::read(file_path)
        .map_err(|e| HimdError::Io(format!("Failed to read audio file for analysis: {e}")))?;

    let duration_sec;
    let rms_energy;

    if let Some(wav) = parse_wav_header(&data) {
        let pcm_data = &data
            [wav.data_offset..wav.data_offset + wav.data_size.min(data.len() - wav.data_offset)];
        rms_energy = compute_rms_energy(pcm_data);
        let total_samples = wav.data_size as f64 / (wav.bits_per_sample as f64 / 8.0);
        duration_sec = total_samples / (wav.sample_rate as f64 * wav.num_channels as f64);
    } else {
        // Not a WAV — rough MP3 estimate (~128kbps = 16KB/sec)
        duration_sec = data.len() as f64 / 16000.0;
        rms_energy = 0.1; // default medium
    }

    let speech_rate = estimate_speech_rate(transcript, duration_sec);
    let energy = classify_energy(rms_energy);
    let pause_pattern = estimate_pause_pattern(transcript, duration_sec);

    // Overall confidence: average of individual confidences
    // (speech_rate 0.7, energy 0.6, pause_pattern 0.5 from Node.js)
    let overall_confidence = (0.7 + 0.6 + 0.5) / 3.0;

    Ok(RawAnalysis {
        speech_rate,
        energy,
        pause_pattern,
        overall_confidence,
    })
}

// ---------------------------------------------------------------------------
// Build audio_turn
// ---------------------------------------------------------------------------

/// Build the final `AudioTurn` by merging provider results with local acoustic analysis.
///
/// This mirrors `buildAudioTurn()` from the Node.js `analyze.js`.
pub fn build_audio_turn(provider_result: &AudioUnderstandingResult, file_path: &str) -> AudioTurn {
    // Run local acoustic analysis
    let raw_analysis =
        analyze_audio(file_path, &provider_result.transcript).unwrap_or(RawAnalysis {
            speech_rate: SpeechRate::Normal,
            energy: EnergyLevel::Medium,
            pause_pattern: PausePattern::Medium,
            overall_confidence: 0.2,
        });

    let analysis = AcousticAnalysis {
        speech_rate: raw_analysis.speech_rate,
        energy: raw_analysis.energy,
        pause_pattern: raw_analysis.pause_pattern,
    };

    // Build enriched audio_understanding if provider returned non-trivial fields
    let has_enrichment = provider_result.summary.is_some()
        || provider_result.intent.is_some()
        || provider_result.emotion.is_some()
        || provider_result.tone.is_some()
        || provider_result.key_points.is_some()
        || provider_result.non_verbal_signals.is_some()
        || provider_result.language.is_some();

    let audio_understanding = if has_enrichment {
        Some(AudioUnderstanding {
            summary: provider_result.summary.clone(),
            intent: provider_result.intent.clone(),
            emotion: provider_result.emotion.as_ref().map(|e| Emotion {
                primary: e.primary.clone(),
                confidence: e.confidence,
            }),
            tone: provider_result.tone.clone(),
            key_points: provider_result.key_points.clone(),
            non_verbal_signals: provider_result.non_verbal_signals.clone(),
            language: provider_result.language.clone(),
            confidence: provider_result.confidence,
        })
    } else {
        None
    };

    let warnings = if provider_result.warnings.is_empty() {
        None
    } else {
        Some(provider_result.warnings.clone())
    };

    AudioTurn {
        transcript: provider_result.transcript.clone(),
        analysis,
        analysis_confidence: raw_analysis.overall_confidence,
        source: "file".to_string(),
        audio_file: file_path.to_string(),
        model: provider_result.model.clone(),
        provider: provider_result.provider.clone(),
        audio_understanding,
        warnings,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_minimal_wav() -> Vec<u8> {
        // Minimal valid WAV: 44-byte header + some PCM data
        let sample_rate: u32 = 16000;
        let num_channels: u16 = 1;
        let bits_per_sample: u16 = 16;
        let num_samples: u32 = 16000; // 1 second of audio
        let data_size = num_samples * (bits_per_sample as u32 / 8) * num_channels as u32;
        let file_size = 36 + data_size;

        let mut wav = Vec::with_capacity(44 + data_size as usize);
        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&file_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        // fmt chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format
        wav.extend_from_slice(&num_channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * num_channels as u32 * (bits_per_sample as u32 / 8);
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        let block_align = num_channels * (bits_per_sample / 8);
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());
        // data chunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());
        // PCM data: alternating values for some energy
        for i in 0..data_size as usize / 2 {
            let sample = if i % 2 == 0 { 10000i16 } else { -10000i16 };
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        wav
    }

    #[test]
    fn parse_wav_header_valid() {
        let wav = make_minimal_wav();
        let info = parse_wav_header(&wav).unwrap();
        assert_eq!(info.num_channels, 1);
        assert_eq!(info.sample_rate, 16000);
        assert_eq!(info.bits_per_sample, 16);
    }

    #[test]
    fn parse_wav_header_too_short() {
        assert!(parse_wav_header(&[0u8; 10]).is_none());
    }

    #[test]
    fn parse_wav_header_not_riff() {
        let mut wav = make_minimal_wav();
        wav[0..4].copy_from_slice(b"NOPE");
        assert!(parse_wav_header(&wav).is_none());
    }

    #[test]
    fn compute_rms_energy_silence() {
        // All zeros = silence
        let pcm: Vec<u8> = vec![0u8; 320]; // 160 samples of 16-bit zero
        let rms = compute_rms_energy(&pcm);
        assert!(rms.abs() < f64::EPSILON);
    }

    #[test]
    fn compute_rms_energy_nonzero() {
        let pcm: Vec<u8> = vec![0xFF, 0x7F]; // 32767 = max
        let rms = compute_rms_energy(&pcm);
        assert!(rms > 0.99);
    }

    #[test]
    fn classify_energy_levels() {
        assert_eq!(classify_energy(0.01), EnergyLevel::Low);
        assert_eq!(classify_energy(0.15), EnergyLevel::Medium);
        assert_eq!(classify_energy(0.50), EnergyLevel::High);
    }

    #[test]
    fn estimate_speech_rate_levels() {
        assert_eq!(estimate_speech_rate("hi", 10.0), SpeechRate::Slow);
        assert_eq!(
            estimate_speech_rate("hello world test speech", 5.0),
            SpeechRate::Normal
        );
        assert_eq!(
            estimate_speech_rate("一二三四五六七八九十一二三四五", 0.5),
            SpeechRate::Fast
        );
    }

    #[test]
    fn estimate_speech_rate_empty_or_zero() {
        assert_eq!(estimate_speech_rate("", 5.0), SpeechRate::Normal);
        assert_eq!(estimate_speech_rate("hello", 0.0), SpeechRate::Normal);
    }

    #[test]
    fn estimate_pause_pattern_levels() {
        // Short: compact speech, audio ~ same as expected
        assert_eq!(estimate_pause_pattern("一二三", 1.0), PausePattern::Short);
        // Medium: moderate pauses
        assert_eq!(estimate_pause_pattern("一二三", 2.0), PausePattern::Medium);
        // Long: many pauses
        assert_eq!(estimate_pause_pattern("一", 5.0), PausePattern::Long);
    }

    #[test]
    fn build_audio_turn_basic() {
        let dir = std::env::temp_dir().join("himd-test-acoustic");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test.wav");
        std::fs::write(&file_path, make_minimal_wav()).unwrap();

        let provider = crate::provider::create_empty_result();
        let turn = build_audio_turn(&provider, file_path.to_str().unwrap());

        assert_eq!(turn.source, "file");
        assert_eq!(turn.audio_file, file_path.to_str().unwrap());
        assert!(turn.audio_understanding.is_none()); // empty provider has no enrichment

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_audio_turn_with_enrichment() {
        let dir = std::env::temp_dir().join("himd-test-acoustic-enriched");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test.wav");
        std::fs::write(&file_path, make_minimal_wav()).unwrap();

        let mut provider = crate::provider::create_empty_result();
        provider.transcript = "你好".to_string();
        provider.summary = Some("greeting".to_string());
        provider.emotion = Some(crate::provider::EmotionResult {
            primary: "happy".to_string(),
            confidence: Some(0.9),
        });

        let turn = build_audio_turn(&provider, file_path.to_str().unwrap());
        assert_eq!(turn.transcript, "你好");
        let au = turn.audio_understanding.unwrap();
        assert_eq!(au.summary.unwrap(), "greeting");
        assert_eq!(au.emotion.unwrap().primary, "happy");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_audio_turn_nonexistent_file_uses_defaults() {
        let provider = crate::provider::create_empty_result();
        let turn = build_audio_turn(&provider, "/tmp/nonexistent_himd_test.wav");
        // Should still produce a valid turn with default analysis
        assert_eq!(turn.analysis.speech_rate, SpeechRate::Normal);
        assert_eq!(turn.analysis.energy, EnergyLevel::Medium);
    }
}
