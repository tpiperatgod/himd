use himd_audio::wav::{build_capture_output_path, write_wav_mono_i16};

#[test]
fn capture_output_path_uses_himd_prefix() {
    let path = build_capture_output_path(1_735_689_600_000);
    let rendered = path.display().to_string();
    assert!(
        rendered.contains("himd"),
        "path should contain 'himd': {rendered}"
    );
    assert!(
        rendered.ends_with(".wav"),
        "path should end with .wav: {rendered}"
    );
}

#[test]
fn wav_writer_emits_riff_header() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.wav");
    write_wav_mono_i16(&path, 16_000, &[0_i16, 1, -1, 2]).unwrap();
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
}
