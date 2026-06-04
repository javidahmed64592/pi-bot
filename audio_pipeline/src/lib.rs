//! Audio Processing Pipeline
//!
//! Speech recognition and synthesis for conversational AI.
//!
//! ## Components
//! - `WakeWordDetector` - Vosk-based wake word detection + speech-to-text
//!   - Dual mode: Wake word detection → Speech capture
//!   - Automatic mode switching after wake word detected
//!   - Configurable timeouts and silence detection
//! - `PiperTts` - Text-to-speech synthesis via Piper engine
//!
//! ## Wake Word + STT Flow
//!
//! 1. **WakeWord Mode**: Listens for configured phrase (e.g., "hey")
//! 2. **Detection**: Switches to SpeechCapture mode automatically
//! 3. **SpeechCapture Mode**: Transcribes until silence or timeout
//! 4. **Return**: Loops back to WakeWord mode with transcription

pub mod tts;
pub mod wake_word;

pub use tts::{PiperTts, TtsError};
pub use wake_word::{DetectorMode, WakeWordDetector, WakeWordError};

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
