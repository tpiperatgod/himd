//! Audio capture utilities.
//!
//! Runtime capture is handled by `himd-audio`. This module retains
//! WAV duration parsing used by acoustic analysis.

use std::path::Path;

// ---------------------------------------------------------------------------
// WAV duration parsing
// ---------------------------------------------------------------------------

/// Parse duration from WAV file header in milliseconds.
pub fn get_wav_duration_ms(path: &Path) -> u64 {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return 0,
    };
    if data.len() < 44 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return 0;
    }
    let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let channels = u16::from_le_bytes([data[22], data[23]]) as u32;
    let bits_per_sample = u16::from_le_bytes([data[34], data[35]]) as u32;

    let mut offset = 12;
    while offset < data.len().saturating_sub(8) {
        let chunk_id = &data[offset..offset + 4];
        let chunk_size = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as u64;
        if chunk_id == b"data" {
            let total_samples = chunk_size / (bits_per_sample / 8) as u64;
            let duration_sec = total_samples as f64 / (sample_rate * channels) as f64;
            return (duration_sec * 1000.0).round() as u64;
        }
        offset += 8 + chunk_size as usize;
    }
    0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_wav_duration_ms_valid() {
        let sample_rate: u32 = 16000;
        let channels: u16 = 1;
        let bits: u16 = 16;
        let num_samples: u32 = 16000;
        let data_size = num_samples * (bits as u32 / 8) * channels as u32;

        let mut wav = Vec::with_capacity(44 + data_size as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_size).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * channels as u32 * (bits as u32 / 8);
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&(channels * (bits / 8)).to_le_bytes());
        wav.extend_from_slice(&bits.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());
        wav.extend(vec![0u8; data_size as usize]);

        let dir = std::env::temp_dir().join("himd-test-wav-duration");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.wav");
        std::fs::write(&path, &wav).unwrap();

        let duration = get_wav_duration_ms(&path);
        assert_eq!(duration, 1000);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn get_wav_duration_ms_nonexistent() {
        assert_eq!(get_wav_duration_ms(Path::new("/tmp/no_such_file.wav")), 0);
    }
}
