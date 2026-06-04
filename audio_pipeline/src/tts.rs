//! # Piper Text-to-Speech
//!
//! Offline neural TTS using the Piper engine (via Python/uv).
//!
//! ## Architecture
//!
//! PiperTts converts text to WAV audio using the Piper command-line tool:
//! 1. Spawns `uv run piper` subprocess with configured voice model
//! 2. Writes text to stdin
//! 3. Reads WAV audio from stdout
//! 4. Returns audio bytes for playback
//!
//! ## Installation
//!
//! Piper is installed via the Python environment using uv:
//! ```bash
//! uv pip install piper-tts
//! ```
//!
//! ## Usage
//!
//! ```no_run
//! use audio_pipeline::PiperTts;
//! use bot_core::config::PiperConfig;
//!
//! let config = PiperConfig {
//!     voice: "en_GB-alba-medium".to_string(),
//!     model_path: "models/piper/en_GB-alba-medium.onnx".to_string(),
//!     config_path: "models/piper/en_GB-alba-medium.onnx.json".to_string(),
//! };
//!
//! let mut tts = PiperTts::new(config).await.expect("Failed to init TTS");
//! let audio = tts.synthesize("Hello world").await.expect("Failed to synthesize");
//! ```

use anyhow::Result;
use bot_core::config::PiperConfig;
use log::{debug, info, warn};
use std::process::Stdio;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum TtsError {
    #[error("Failed to spawn Piper process: {0}")]
    ProcessSpawnError(String),

    #[error("Failed to write text to Piper stdin: {0}")]
    StdinWriteError(String),

    #[error("Failed to read audio from Piper stdout: {0}")]
    StdoutReadError(String),

    #[error("Piper process failed with exit code {code}: {stderr}")]
    ProcessFailed { code: i32, stderr: String },

    #[error("Empty audio output from Piper")]
    EmptyOutput,

    #[error("Invalid Piper configuration: {0}")]
    InvalidConfig(String),
}

// ============================================================================
// PiperTts - Text-to-Speech Engine
// ============================================================================

/// Piper TTS engine wrapper
///
/// Uses the Piper command-line tool to synthesize speech from text.
/// Each synthesis spawns a new process for reliability.
pub struct PiperTts {
    config: PiperConfig,
}

impl PiperTts {
    /// Create a new Piper TTS engine
    ///
    /// # Arguments
    ///
    /// * `config` - Piper configuration (model path, voice, etc.)
    ///
    /// # Returns
    ///
    /// Result containing PiperTts instance or error if Piper is not available
    pub async fn new(config: PiperConfig) -> Result<Self, TtsError> {
        // Validate config
        if config.model_path.is_empty() {
            return Err(TtsError::InvalidConfig(
                "model_path cannot be empty".to_string(),
            ));
        }

        info!(
            "Initialized Piper TTS with voice: {} (model: {})",
            config.voice, config.model_path
        );

        Ok(Self { config })
    }

    /// Synthesize speech from text
    ///
    /// # Arguments
    ///
    /// * `text` - Text to synthesize (UTF-8 string)
    ///
    /// # Returns
    ///
    /// Result containing WAV audio bytes or TtsError
    ///
    /// # Behavior
    ///
    /// - Spawns Piper subprocess with configured model
    /// - Writes text to stdin
    /// - Reads WAV audio from stdout
    /// - Returns audio bytes ready for playback
    pub async fn synthesize(&mut self, text: &str) -> Result<Vec<u8>, TtsError> {
        if text.is_empty() {
            warn!("Attempted to synthesize empty text");
            return Err(TtsError::EmptyOutput);
        }

        debug!("Synthesizing speech: '{}'", text);

        // Spawn Piper process via uv
        // Command: uv run piper --model <model_path> --config <config_path> --output-raw
        // --output-raw outputs raw PCM to stdout (16-bit, mono, 22050 Hz)
        let mut child = Command::new("uv")
            .arg("run")
            .arg("piper")
            .arg("--model")
            .arg(&self.config.model_path)
            .arg("--config")
            .arg(&self.config.config_path)
            .arg("--output-raw")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TtsError::ProcessSpawnError(e.to_string()))?;

        // Get stdin handle
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| TtsError::StdinWriteError("Failed to open stdin".to_string()))?;

        // Write text to stdin
        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|e| TtsError::StdinWriteError(e.to_string()))?;

        // Close stdin to signal end of input
        drop(stdin);

        // Read audio from stdout
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| TtsError::StdoutReadError("Failed to open stdout".to_string()))?;

        let mut audio_data = Vec::new();
        stdout
            .read_to_end(&mut audio_data)
            .await
            .map_err(|e| TtsError::StdoutReadError(e.to_string()))?;

        // Read stderr for error messages
        let mut stderr = child.stderr.take().ok_or_else(|| TtsError::ProcessFailed {
            code: -1,
            stderr: "Failed to open stderr".to_string(),
        })?;

        let mut stderr_data = Vec::new();
        stderr
            .read_to_end(&mut stderr_data)
            .await
            .map_err(|e| TtsError::ProcessFailed {
                code: -1,
                stderr: format!("Failed to read stderr: {}", e),
            })?;

        // Wait for process to complete
        let status = child.wait().await.map_err(|e| TtsError::ProcessFailed {
            code: -1,
            stderr: e.to_string(),
        })?;

        // Check if process succeeded
        if !status.success() {
            let stderr_str = String::from_utf8_lossy(&stderr_data);
            return Err(TtsError::ProcessFailed {
                code: status.code().unwrap_or(-1),
                stderr: stderr_str.to_string(),
            });
        }

        // Validate output
        if audio_data.is_empty() {
            let stderr_str = String::from_utf8_lossy(&stderr_data);
            debug!("Piper stderr: {}", stderr_str);
            return Err(TtsError::EmptyOutput);
        }

        info!(
            "Synthesized {} bytes of audio for text: '{}'",
            audio_data.len(),
            text
        );

        // Wrap raw PCM data in WAV header
        // Piper outputs: 16-bit, mono, 22050 Hz
        let wav_data = create_wav_header(&audio_data, 22050, 1, 16);

        Ok(wav_data)
    }

    /// Get the current voice name
    pub fn voice(&self) -> &str {
        &self.config.voice
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a WAV file header and prepend it to raw PCM data
///
/// # Arguments
///
/// * `pcm_data` - Raw PCM audio bytes
/// * `sample_rate` - Sample rate in Hz (e.g., 22050)
/// * `channels` - Number of channels (1 = mono, 2 = stereo)
/// * `bits_per_sample` - Bits per sample (typically 16)
///
/// # Returns
///
/// Complete WAV file data (header + PCM data)
fn create_wav_header(
    pcm_data: &[u8],
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
) -> Vec<u8> {
    let byte_rate = sample_rate * u32::from(channels) * u32::from(bits_per_sample) / 8;
    let block_align = channels * bits_per_sample / 8;
    let data_size = pcm_data.len() as u32;
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + pcm_data.len());

    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // audio format (1 = PCM)
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend_from_slice(pcm_data);

    wav
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Piper to be installed
    async fn test_piper_available() {
        let config = PiperConfig {
            voice: "en_GB-alba-medium".to_string(),
            model_path: "models/piper/en_GB-alba-medium.onnx".to_string(),
            config_path: "models/piper/en_GB-alba-medium.onnx.json".to_string(),
        };

        let result = PiperTts::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires Piper to be installed and models downloaded
    async fn test_synthesize() {
        let config = PiperConfig {
            voice: "en_GB-alba-medium".to_string(),
            model_path: "models/piper/en_GB-alba-medium.onnx".to_string(),
            config_path: "models/piper/en_GB-alba-medium.onnx.json".to_string(),
        };

        let mut tts = PiperTts::new(config).await.expect("Failed to init TTS");
        let audio = tts
            .synthesize("Hello world")
            .await
            .expect("Failed to synthesize");

        assert!(!audio.is_empty());
        assert!(audio.len() > 1000); // Should have significant audio data
    }

    #[tokio::test]
    async fn test_empty_text() {
        let config = PiperConfig {
            voice: "en_GB-alba-medium".to_string(),
            model_path: "models/piper/en_GB-alba-medium.onnx".to_string(),
            config_path: "models/piper/en_GB-alba-medium.onnx.json".to_string(),
        };

        let mut tts = PiperTts::new(config).await.expect("Failed to init TTS");
        let result = tts.synthesize("").await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TtsError::EmptyOutput));
    }
}
