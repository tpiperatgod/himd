use himd_audio::vad::{StopReason, VadConfig, VadState};

#[test]
fn silence_before_speech_becomes_no_speech() {
    let mut vad = VadState::new(VadConfig::default());
    let stop = vad.push_energy(0.0, 8_100);
    assert_eq!(stop, Some(StopReason::NoSpeech));
}

#[test]
fn speech_then_long_silence_becomes_silence() {
    let mut vad = VadState::new(VadConfig::default());
    assert_eq!(vad.push_energy(0.30, 500), None);
    let stop = vad.push_energy(0.0, 2_100);
    assert_eq!(stop, Some(StopReason::Silence));
}

#[test]
fn timeout_wins_when_max_duration_is_hit() {
    let mut vad = VadState::new(VadConfig::default());
    let stop = vad.push_elapsed(30_000);
    assert_eq!(stop, Some(StopReason::Timeout));
}
