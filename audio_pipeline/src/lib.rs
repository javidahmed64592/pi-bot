//! # audio_pipeline
//!
//! Speech-to-text and text-to-speech pipeline.
//!
//! Components:
//! - wake_word.rs: Vosk-based wake word detection ("Hey Bot")
//! - stt.rs: Vosk speech-to-text (same engine as wake word)
//! - tts.rs: Piper text-to-speech synthesis (via tokio::process::Command)

pub mod wake_word;

pub use wake_word::{WakeWordDetector, WakeWordError};

// TODO: Phase 1.7 - Implement stt.rs with Vosk full recognition
// TODO: Phase 1.10 - Implement tts.rs with Piper (use tokio::process::Command)

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
