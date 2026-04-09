#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    NoSpeech,
    Silence,
    Timeout,
}

#[derive(Debug, Clone, Copy)]
pub struct VadConfig {
    pub no_speech_ms: u64,
    pub silence_after_speech_ms: u64,
    pub max_duration_ms: u64,
    pub speech_threshold: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            no_speech_ms: 8_000,
            silence_after_speech_ms: 1_500,
            max_duration_ms: 30_000,
            speech_threshold: 0.02,
        }
    }
}

#[derive(Debug)]
pub struct VadState {
    config: VadConfig,
    speech_seen: bool,
    elapsed_ms: u64,
    silence_since_speech_ms: u64,
}

impl VadState {
    pub fn new(config: VadConfig) -> Self {
        Self {
            config,
            speech_seen: false,
            elapsed_ms: 0,
            silence_since_speech_ms: 0,
        }
    }

    /// Push an energy reading with a duration in milliseconds.
    /// Returns `Some(StopReason)` if the capture should stop.
    pub fn push_energy(&mut self, energy: f32, duration_ms: u64) -> Option<StopReason> {
        self.elapsed_ms += duration_ms;

        if self.elapsed_ms >= self.config.max_duration_ms {
            return Some(StopReason::Timeout);
        }

        if energy >= self.config.speech_threshold {
            self.speech_seen = true;
            self.silence_since_speech_ms = 0;
            None
        } else if self.speech_seen {
            self.silence_since_speech_ms += duration_ms;
            if self.silence_since_speech_ms >= self.config.silence_after_speech_ms {
                Some(StopReason::Silence)
            } else {
                None
            }
        } else {
            // No speech yet — check grace period
            if self.elapsed_ms >= self.config.no_speech_ms {
                Some(StopReason::NoSpeech)
            } else {
                None
            }
        }
    }

    /// Push only elapsed time (no energy data). Used for timeout checks.
    pub fn push_elapsed(&mut self, total_elapsed_ms: u64) -> Option<StopReason> {
        self.elapsed_ms = total_elapsed_ms;
        if self.elapsed_ms >= self.config.max_duration_ms {
            Some(StopReason::Timeout)
        } else {
            None
        }
    }

    pub fn speech_detected(&self) -> bool {
        self.speech_seen
    }
}
