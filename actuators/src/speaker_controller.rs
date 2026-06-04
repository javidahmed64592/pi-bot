//! # Speaker Controller
//!
//! Audio playback controller for TTS output.
//!
//! ## Architecture
//!
//! SpeakerController is a **dumb actuator** that only plays audio:
//! - Receives raw PCM audio bytes from Piper TTS
//! - Plays audio through the configured speaker device
//! - Can stop playback on command
//! - No business logic, no decision making
//!
//! ## Usage
//!
//! ```no_run
//! use actuators::SpeakerController;
//!
//! let mut controller = SpeakerController::new("default")
//!     .expect("Failed to initialize speaker");
//!
//! // Play raw PCM audio (16-bit, mono, 22050 Hz - Piper default)
//! let audio_data: Vec<u8> = vec![/* raw PCM bytes */];
//! controller.play(&audio_data).expect("Failed to play audio");
//!
//! // Stop current playback
//! controller.stop();
//! ```

use anyhow::Result;
use log::{debug, info, warn};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum SpeakerError {
    #[error("Failed to initialize audio output: {0}")]
    OutputInitError(String),

    #[error("Failed to decode audio: {0}")]
    DecodeError(String),

    #[error("Failed to play audio: {0}")]
    PlaybackError(String),

    #[error("No audio sink available")]
    NoSink,
}

// ============================================================================
// SpeakerController - Audio Playback Actuator
// ============================================================================

/// Speaker controller for playing TTS audio
///
/// Handles audio output using the rodio library. Manages playback state
/// and allows stopping current playback.
pub struct SpeakerController {
    /// Audio output stream (must be kept alive)
    _stream: OutputStream,

    /// Audio output stream handle for creating sinks
    stream_handle: OutputStreamHandle,

    /// Current playback sink (if any)
    current_sink: Arc<Mutex<Option<Sink>>>,

    /// Device name for logging
    device_name: String,
}

impl SpeakerController {
    /// Create a new speaker controller
    ///
    /// # Arguments
    ///
    /// * `device_name` - Audio device name (e.g., "default")
    ///
    /// # Returns
    ///
    /// Result containing SpeakerController or error if device unavailable
    pub fn new(device_name: &str) -> Result<Self, SpeakerError> {
        // Initialize audio output stream
        let (stream, stream_handle) = OutputStream::try_default().map_err(|e| {
            SpeakerError::OutputInitError(format!("Failed to open audio device: {}", e))
        })?;

        info!("Initialized speaker controller on device: {}", device_name);

        Ok(Self {
            _stream: stream,
            stream_handle,
            current_sink: Arc::new(Mutex::new(None)),
            device_name: device_name.to_string(),
        })
    }

    /// Play raw PCM audio
    ///
    /// # Arguments
    ///
    /// * `audio_data` - Raw PCM audio bytes (Piper outputs 16-bit mono at 22050 Hz)
    ///
    /// # Behavior
    ///
    /// - Stops any currently playing audio
    /// - Creates new sink and plays audio
    /// - Blocks until playback starts (not until completion)
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn play(&mut self, audio_data: &[u8]) -> Result<(), SpeakerError> {
        if audio_data.is_empty() {
            warn!("Attempted to play empty audio data");
            return Ok(());
        }

        debug!("Playing {} bytes of audio", audio_data.len());

        // Stop any current playback
        self.stop();

        // Create new sink
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| SpeakerError::PlaybackError(format!("Failed to create sink: {}", e)))?;

        // Decode audio from bytes
        // Piper outputs raw PCM, but we need to wrap it in a WAV container for rodio
        // For now, we'll try to decode it as-is (Piper can output WAV with --output_raw flag)
        let cursor = Cursor::new(audio_data.to_vec());
        let source = Decoder::new(cursor)
            .map_err(|e| SpeakerError::DecodeError(format!("Failed to decode audio: {}", e)))?;

        // Append to sink and play
        sink.append(source);

        // Store sink reference
        {
            let mut current = self.current_sink.lock().unwrap();
            *current = Some(sink);
        }

        info!("Started audio playback on {}", self.device_name);

        Ok(())
    }

    /// Play WAV audio from file-like bytes
    ///
    /// # Arguments
    ///
    /// * `wav_data` - Complete WAV file data (with header)
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn play_wav(&mut self, wav_data: &[u8]) -> Result<(), SpeakerError> {
        if wav_data.is_empty() {
            warn!("Attempted to play empty WAV data");
            return Ok(());
        }

        debug!("Playing {} bytes of WAV audio", wav_data.len());

        // Stop any current playback
        self.stop();

        // Create new sink
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| SpeakerError::PlaybackError(format!("Failed to create sink: {}", e)))?;

        // Decode WAV audio
        let cursor = Cursor::new(wav_data.to_vec());
        let source = Decoder::new(cursor)
            .map_err(|e| SpeakerError::DecodeError(format!("Failed to decode WAV: {}", e)))?;

        // Append to sink and play
        sink.append(source);

        // Store sink reference
        {
            let mut current = self.current_sink.lock().unwrap();
            *current = Some(sink);
        }

        info!("Started WAV audio playback on {}", self.device_name);

        Ok(())
    }

    /// Stop current audio playback
    ///
    /// # Behavior
    ///
    /// - Immediately stops any playing audio
    /// - Clears the current sink
    /// - Safe to call even if nothing is playing
    pub fn stop(&mut self) {
        let mut current = self.current_sink.lock().unwrap();
        if let Some(sink) = current.take() {
            sink.stop();
            debug!("Stopped audio playback");
        }
    }

    /// Check if audio is currently playing
    ///
    /// # Returns
    ///
    /// true if audio is playing, false otherwise
    pub fn is_playing(&self) -> bool {
        let current = self.current_sink.lock().unwrap();
        current.as_ref().map_or(false, |sink| !sink.empty())
    }

    /// Wait for current playback to finish
    ///
    /// # Behavior
    ///
    /// Blocks until the current audio finishes playing.
    /// Returns immediately if nothing is playing.
    pub fn wait_for_completion(&self) {
        let current = self.current_sink.lock().unwrap();
        if let Some(sink) = current.as_ref() {
            sink.sleep_until_end();
            debug!("Audio playback completed");
        }
    }

    /// Get the device name
    pub fn device_name(&self) -> &str {
        &self.device_name
    }
}

impl Drop for SpeakerController {
    fn drop(&mut self) {
        self.stop();
        info!("Speaker controller dropped");
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires audio hardware
    fn test_speaker_init() {
        let result = SpeakerController::new("default");
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_audio() {
        let mut controller = SpeakerController::new("default").unwrap();
        let result = controller.play(&[]);
        assert!(result.is_ok()); // Should not error, just skip
    }
}
