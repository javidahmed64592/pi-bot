//! # audio_pipeline
//!
//! Speech-to-text and text-to-speech pipeline.
//!
//! Components:
//! - wake_word.rs: Vosk-based wake word detection + speech-to-text
//!   - Dual mode operation: WakeWord detection → Speech capture
//!   - Automatic mode switching after wake word detected
//!   - Configurable timeouts and silence detection
//! - tts.rs: Piper text-to-speech synthesis (TODO: Phase 1.8)
//!
//! ## Wake Word + STT Flow
//!
//! 1. WakeWord Mode: Listens for configured wake phrase (e.g., "hey")
//! 2. When detected: Automatically switches to SpeechCapture mode
//! 3. SpeechCapture Mode: Transcribes user speech until:
//!    - Silence detected (configurable threshold)
//!    - Timeout reached (configurable limit)
//! 4. Returns to WakeWord mode with captured transcription

pub mod wake_word;

pub use wake_word::{DetectorMode, WakeWordDetector, WakeWordError};

// TODO: Phase 1.8 - Implement tts.rs with Piper (use tokio::process::Command)

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
