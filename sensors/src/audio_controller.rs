//! Audio Controller for Wake Word Detection
//!
//! This controller manages the wake word detection system and emits
//! Event::WakeWordDetected when "hey bot" is heard.
//!
//! # Architecture
//! - Uses audio_pipeline::WakeWordDetector for Vosk-based detection
//! - Polls for wake word detection in async context
//! - Emits events to the controller via channels
//!
//! # Example
//! ```ignore
//! use bot_core::{Event, load_config};
//! use sensors::AudioController;
//!
//! # fn example() -> anyhow::Result<()> {
//! let config = load_config("config/config.yaml")?;
//! let mut audio = AudioController::new(&config, "AudioSensor")?;
//!
//! // Start listening
//! audio.start()?;
//!
//! // Poll for wake word
//! if let Some(event) = audio.poll() {
//!     println!("Event: {:?}", event);
//! }
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use audio_pipeline::WakeWordDetector;
use bot_core::{Event, SystemConfig};
use thiserror::Error;

/// Errors that can occur with the audio controller
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Failed to initialize wake word detector: {0}")]
    WakeWordInitError(String),

    #[error("Audio controller not started")]
    NotStarted,
}

/// Audio controller for wake word detection
///
/// Manages wake word detection and emits Event::WakeWordDetected
pub struct AudioController {
    /// Wake word detector
    detector: WakeWordDetector,

    /// Label for logging
    label: String,

    /// Whether the detector is currently listening
    is_listening: bool,
}

impl AudioController {
    /// Create a new audio controller
    ///
    /// # Arguments
    /// * `config` - System configuration with audio settings
    /// * `label` - Label for logging (e.g., "AudioSensor")
    ///
    /// # Returns
    /// New AudioController instance (not yet started)
    pub fn new(config: &SystemConfig, label: &str) -> Result<Self> {
        log::info!("[{}] Initializing audio controller", label);

        let audio_config = &config.audio;

        // Create wake word detector
        let detector = WakeWordDetector::new(
            &audio_config.vosk.model_path,
            &audio_config.vosk.wake_phrase,
            audio_config.sample_rate,
            audio_config.vosk.stt.clone(),
        )
        .map_err(|e| AudioError::WakeWordInitError(e.to_string()))?;

        log::info!("[{}] Wake word detector initialized", label);

        Ok(Self {
            detector,
            label: label.to_string(),
            is_listening: false,
        })
    }

    /// Start the audio controller
    ///
    /// Begins listening for the wake word. Must be called before polling.
    ///
    /// # Errors
    /// Returns error if audio stream cannot be started
    pub fn start(&mut self) -> Result<()> {
        log::info!("[{}] Starting audio controller", self.label);

        self.detector.start_listening()?;
        self.is_listening = true;

        log::info!("[{}] Audio controller started", self.label);

        Ok(())
    }

    /// Poll for wake word detection
    ///
    /// # Returns
    /// - `Some(Event::WakeWordDetected)` if wake word was detected
    /// - `None` if no wake word detected
    ///
    /// # Behavior
    /// This is a non-blocking poll. Call this repeatedly in your event loop.
    /// When wake word is detected, automatically switches to speech capture mode.
    pub fn poll(&mut self) -> Option<Event> {
        if !self.is_listening {
            return None;
        }

        // Check for wake word
        if self.detector.check_for_wake_word() {
            log::info!(
                "[{}] Wake word detected, switching to speech capture",
                self.label
            );
            // Automatically start speech capture
            self.detector.start_speech_capture();
            return Some(Event::WakeWordDetected);
        }

        // Check for captured speech (if in speech capture mode)
        if let Some(speech) = self.detector.check_for_captured_speech() {
            log::info!("[{}] Speech captured: '{}'", self.label, speech);
            return Some(Event::SpeechCaptured(speech));
        }

        None
    }

    /// Stop the audio controller
    ///
    /// Stops listening for the wake word
    pub fn stop(&mut self) {
        log::info!("[{}] Stopping audio controller", self.label);
        self.detector.stop_listening();
        self.is_listening = false;
    }

    /// Check if the controller is currently listening
    pub fn is_listening(&self) -> bool {
        self.is_listening
    }

    /// Get the last transcription (for debugging)
    pub fn last_transcription(&self) -> Option<String> {
        self.detector.last_transcription()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        // Actual audio testing requires hardware
        assert!(true);
    }
}
