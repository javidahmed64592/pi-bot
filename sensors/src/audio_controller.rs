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
//! ```no_run
//! use bot_core::{Event, load_config};
//! use sensors::AudioController;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = load_config("config/config.yaml")?;
//! let mut audio = AudioController::new(&config.audio, "AudioSensor")?;
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

    /// Previous detector mode (to detect transitions)
    previous_mode: audio_pipeline::DetectorMode,

    /// Flag to track if we've sent SpeechCaptureStarted event
    speech_capture_started_sent: bool,
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
            previous_mode: audio_pipeline::DetectorMode::WakeWord,
            speech_capture_started_sent: false,
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
    /// - `Some(Event::SpeechCaptureStarted)` if speech capture has started
    /// - `Some(Event::SpeechCaptured(text))` if speech was captured
    /// - `None` if no wake word detected
    ///
    /// # Behavior
    /// This is a non-blocking poll. Call this repeatedly in your event loop.
    /// When wake word is detected, automatically switches to speech capture mode.
    pub fn poll(&mut self) -> Option<Event> {
        if !self.is_listening {
            return None;
        }

        // Only check for wake word when in WakeWord mode — prevents a stale
        // detection buffered in the background thread from firing again after
        // start_speech_capture() has already switched the mode.
        if self.detector.mode() == audio_pipeline::DetectorMode::WakeWord
            && self.detector.check_for_wake_word()
        {
            log::info!(
                "[{}] Wake word detected, switching to speech capture",
                self.label
            );
            // Automatically start speech capture
            self.detector.start_speech_capture();
            self.speech_capture_started_sent = false; // Reset flag
            return Some(Event::WakeWordDetected);
        }

        // Check if we transitioned to speech capture mode and haven't sent the event yet
        let current_mode = self.detector.mode();
        if current_mode == audio_pipeline::DetectorMode::SpeechCapture
            && !self.speech_capture_started_sent
        {
            log::info!("[{}] Speech capture mode active", self.label);
            self.speech_capture_started_sent = true;
            self.previous_mode = current_mode;
            return Some(Event::SpeechCaptureStarted);
        }

        // Check for captured speech (if in speech capture mode)
        if let Some(speech) = self.detector.check_for_captured_speech() {
            log::info!("[{}] Speech captured: '{}'", self.label, speech);
            self.speech_capture_started_sent = false; // Reset for next wake word
            self.previous_mode = audio_pipeline::DetectorMode::WakeWord;
            return Some(Event::SpeechCaptured(speech));
        }

        // Update previous mode
        self.previous_mode = current_mode;

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

    /// Temporarily disable audio detection
    ///
    /// This keeps the audio stream running but prevents wake word detection
    /// and speech transcription. Used when the bot is thinking or speaking
    /// to prevent it from listening to itself.
    ///
    /// Always resets the detector state, even if already disabled, because the
    /// Vosk background thread continues accumulating detections while disabled.
    pub fn disable_detection(&mut self) {
        if self.is_listening {
            log::info!("[{}] Disabling audio detection", self.label);
        }
        self.is_listening = false;

        // Stop the processing thread from feeding audio into Vosk.
        // Must be done before reset so no new detections sneak in.
        self.detector.set_detection_enabled(false);

        // Flush any state that built up just before we disabled.
        self.detector.reset_to_wake_word_mode();
        self.speech_capture_started_sent = false;
        self.previous_mode = audio_pipeline::DetectorMode::WakeWord;
    }

    /// Re-enable audio detection
    ///
    /// Resumes wake word detection and speech transcription after being disabled.
    /// Resets detector state first to flush any detections that may have arrived
    /// between the disable call and now, then re-opens the gate in the processing
    /// thread so Vosk starts consuming audio again.
    pub fn enable_detection(&mut self) {
        if !self.is_listening {
            log::info!("[{}] Enabling audio detection", self.label);
            // Flush any stale state before letting the thread process again.
            self.detector.reset_to_wake_word_mode();
            self.speech_capture_started_sent = false;
            self.previous_mode = audio_pipeline::DetectorMode::WakeWord;
            // Re-open the gate in the background thread.
            self.detector.set_detection_enabled(true);
            self.is_listening = true;
        }
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
