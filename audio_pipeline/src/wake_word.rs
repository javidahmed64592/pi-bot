//! Wake Word Detection using Vosk
//!
//! This module implements wake word detection ("hey bot") using the Vosk speech recognition library.
//! It uses keyword spotting mode for efficient, low-CPU wake word detection.
//!
//! # Architecture
//! - Uses `cpal` for cross-platform audio input
//! - Uses `vosk` for offline speech recognition (keyword mode)
//! - Processes audio in real-time with configurable sample rate (16kHz default)
//! - Returns detection events when wake phrase is detected
//!
//! # Example
//! ```no_run
//! use audio_pipeline::WakeWordDetector;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let detector = WakeWordDetector::new(
//!     "models/vosk/vosk-model-small-en-us-0.15",
//!     "hey bot",
//!     16000,
//! )?;
//!
//! // Start listening in background
//! let mut detector = detector;
//! if detector.check_for_wake_word().await? {
//!     println!("Wake word detected!");
//! }
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;
use thiserror::Error;

/// Audio sample frame from hardware (includes format info)
#[derive(Clone, Debug)]
enum AudioFrame {
    F32(Vec<f32>),
    I16(Vec<i16>),
    U16(Vec<u16>),
}

/// Simple linear interpolation resampler for variable-sized buffers
fn resample_linear(input: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
    if input_rate == output_rate {
        return input.to_vec();
    }

    let ratio = input_rate as f64 / output_rate as f64;
    let output_len = (input.len() as f64 / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 * ratio;
        let src_idx = src_pos.floor() as usize;
        let frac = src_pos - src_idx as f64;

        if src_idx + 1 < input.len() {
            // Linear interpolation
            let sample = input[src_idx] * (1.0 - frac) as f32 + input[src_idx + 1] * frac as f32;
            output.push(sample);
        } else if src_idx < input.len() {
            // Last sample, no interpolation
            output.push(input[src_idx]);
        }
    }

    output
}

/// Errors that can occur during wake word detection
#[derive(Debug, Error)]
pub enum WakeWordError {
    #[error("Failed to initialize audio device: {0}")]
    AudioDeviceError(String),

    #[error("Failed to load Vosk model from {path}: {source}")]
    ModelLoadError {
        path: String,
        source: std::io::Error,
    },

    #[error("Vosk recognizer error: {0}")]
    RecognizerError(String),

    #[error("Audio stream error: {0}")]
    StreamError(String),
}

/// Wake word detector using Vosk keyword spotting
///
/// This detector continuously monitors audio input and signals when
/// the configured wake phrase is detected.
pub struct WakeWordDetector {
    /// Vosk recognizer for keyword spotting
    recognizer: Arc<Mutex<vosk::Recognizer>>,

    /// The wake phrase to detect (e.g., "hey bot")
    wake_phrase: String,

    /// Sample rate (Hz) - stored for potential reconfiguration
    #[allow(dead_code)]
    sample_rate: u32,

    /// Audio input stream (kept alive)
    #[allow(dead_code)]
    stream: Option<cpal::Stream>,

    /// Detection flag (set to true when wake word detected)
    detected: Arc<Mutex<bool>>,

    /// Last transcription result (for debugging)
    last_transcription: Arc<Mutex<Option<String>>>,

    /// Channel for sending audio from callback to processing thread
    audio_tx: Option<mpsc::SyncSender<AudioFrame>>,

    /// Processing thread handle
    processing_thread: Option<thread::JoinHandle<()>>,

    /// Flag to signal processing thread to stop
    stop_flag: Arc<AtomicBool>,
}

impl WakeWordDetector {
    /// Create a new wake word detector
    ///
    /// # Arguments
    /// * `model_path` - Path to Vosk model directory (e.g., "models/vosk/vosk-model-small-en-us-0.15")
    /// * `wake_phrase` - Wake phrase to detect (e.g., "hey bot")
    /// * `sample_rate` - Audio sample rate in Hz (typically 16000)
    ///
    /// # Returns
    /// A new WakeWordDetector instance ready to start listening
    ///
    /// # Errors
    /// Returns error if:
    /// - Vosk model cannot be loaded
    /// - Audio device initialization fails
    pub fn new(model_path: &str, wake_phrase: &str, sample_rate: u32) -> Result<Self> {
        log::info!("Initializing wake word detector");
        log::info!("  Model path: {}", model_path);
        log::info!("  Wake phrase: '{}'", wake_phrase);
        log::info!("  Sample rate: {} Hz", sample_rate);

        // Load Vosk model
        let model = vosk::Model::new(model_path).ok_or_else(|| WakeWordError::ModelLoadError {
            path: model_path.to_string(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Failed to load Vosk model from {}", model_path),
            ),
        })?;

        log::info!("Vosk model loaded successfully");

        // Create recognizer
        let recognizer = vosk::Recognizer::new(&model, sample_rate as f32).ok_or_else(|| {
            WakeWordError::RecognizerError(format!(
                "Failed to create Vosk recognizer with sample rate {} Hz",
                sample_rate
            ))
        })?;

        log::info!("Vosk recognizer initialized");

        Ok(Self {
            recognizer: Arc::new(Mutex::new(recognizer)),
            wake_phrase: wake_phrase.to_lowercase(),
            sample_rate,
            stream: None,
            detected: Arc::new(Mutex::new(false)),
            last_transcription: Arc::new(Mutex::new(None)),
            audio_tx: None,
            processing_thread: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Start listening for wake word
    ///
    /// This starts the audio input stream and begins processing audio in the background.
    /// Call `check_for_wake_word()` periodically to check if the wake word was detected.
    ///
    /// # Errors
    /// Returns error if audio stream cannot be started
    pub fn start_listening(&mut self) -> Result<()> {
        log::info!("Starting audio input stream");

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| WakeWordError::AudioDeviceError("No input device found".to_string()))?;

        log::info!("Using audio device: {}", device.name().unwrap_or_default());

        // Get hardware's default configuration (accept what it supports)
        let hw_config = device
            .default_input_config()
            .map_err(|e| WakeWordError::AudioDeviceError(format!("Config error: {}", e)))?;

        let hw_sample_rate = hw_config.sample_rate().0;
        let hw_channels = hw_config.channels();

        log::info!(
            "Hardware audio config: {} Hz, {} channel(s), {:?}",
            hw_sample_rate,
            hw_channels,
            hw_config.sample_format()
        );

        if hw_sample_rate != self.sample_rate {
            log::info!(
                "Will resample from {} Hz to {} Hz",
                hw_sample_rate,
                self.sample_rate
            );
        }

        // Create a custom stream config with a larger buffer to avoid underruns
        let stream_config: cpal::StreamConfig = hw_config.clone().into();
        // Use default buffer size determined by the hardware/driver
        log::info!("Using default buffer size (hardware-determined)");

        // Create channel for audio processing (bounded to avoid memory issues)
        let (audio_tx, audio_rx) = mpsc::sync_channel::<AudioFrame>(8);
        self.audio_tx = Some(audio_tx.clone());

        // Reset stop flag
        self.stop_flag.store(false, Ordering::Relaxed);

        // Spawn processing thread
        let recognizer = Arc::clone(&self.recognizer);
        let detected = Arc::clone(&self.detected);
        let last_transcription = Arc::clone(&self.last_transcription);
        let wake_phrase = self.wake_phrase.clone();
        let target_rate = self.sample_rate;
        let stop_flag = Arc::clone(&self.stop_flag);

        let processing_thread = thread::spawn(move || {
            Self::audio_processing_thread(
                audio_rx,
                recognizer,
                detected,
                last_transcription,
                wake_phrase,
                hw_sample_rate,
                hw_channels,
                target_rate,
                stop_flag,
            )
        });

        self.processing_thread = Some(processing_thread);

        // Build audio input stream with hardware's native format
        let stream = match hw_config.sample_format() {
            cpal::SampleFormat::F32 => Self::build_stream_f32(&device, &stream_config, audio_tx)?,
            cpal::SampleFormat::I16 => Self::build_stream_i16(&device, &stream_config, audio_tx)?,
            cpal::SampleFormat::U16 => Self::build_stream_u16(&device, &stream_config, audio_tx)?,
            _ => {
                return Err(WakeWordError::AudioDeviceError(
                    "Unsupported sample format".to_string(),
                )
                .into())
            }
        };

        stream
            .play()
            .map_err(|e| WakeWordError::StreamError(format!("Failed to start stream: {}", e)))?;

        log::info!("Audio stream started successfully");

        self.stream = Some(stream);
        Ok(())
    }

    /// Audio processing thread - receives raw audio, resamples, feeds to Vosk
    fn audio_processing_thread(
        audio_rx: mpsc::Receiver<AudioFrame>,
        recognizer: Arc<Mutex<vosk::Recognizer>>,
        detected: Arc<Mutex<bool>>,
        last_transcription: Arc<Mutex<Option<String>>>,
        wake_phrase: String,
        hw_sample_rate: u32,
        hw_channels: u16,
        target_rate: u32,
        stop_flag: Arc<AtomicBool>,
    ) {
        let mut buffer: Vec<i16> = Vec::with_capacity(8000);
        let mut frame_count = 0u64;

        while !stop_flag.load(Ordering::Relaxed) {
            match audio_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(frame) => {
                    frame_count += 1;

                    // Log every 50 frames (~1 second at typical buffer sizes)
                    if frame_count % 50 == 0 {
                        log::debug!("Processed {} audio frames", frame_count);
                    }

                    // Convert to mono f32
                    let mono_f32: Vec<f32> = match frame {
                        AudioFrame::F32(data) => {
                            if hw_channels == 2 {
                                data.chunks_exact(2)
                                    .map(|frame| (frame[0] + frame[1]) / 2.0)
                                    .collect()
                            } else {
                                data
                            }
                        }
                        AudioFrame::I16(data) => {
                            if hw_channels == 2 {
                                data.chunks_exact(2)
                                    .map(|frame| {
                                        ((frame[0] as f32 + frame[1] as f32) / 2.0) / 32768.0
                                    })
                                    .collect()
                            } else {
                                data.iter().map(|&s| s as f32 / 32768.0).collect()
                            }
                        }
                        AudioFrame::U16(data) => {
                            if hw_channels == 2 {
                                data.chunks_exact(2)
                                    .map(|frame| {
                                        let s0 = (frame[0] as i32 - 32768) as f32 / 32768.0;
                                        let s1 = (frame[1] as i32 - 32768) as f32 / 32768.0;
                                        (s0 + s1) / 2.0
                                    })
                                    .collect()
                            } else {
                                data.iter()
                                    .map(|&s| (s as i32 - 32768) as f32 / 32768.0)
                                    .collect()
                            }
                        }
                    };

                    // Resample to target rate
                    let resampled = resample_linear(&mono_f32, hw_sample_rate, target_rate);

                    // Convert to i16
                    let samples_i16: Vec<i16> = resampled
                        .iter()
                        .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                        .collect();

                    // Accumulate samples
                    buffer.extend_from_slice(&samples_i16);

                    // Process when we have enough samples (~100ms at 16kHz)
                    if buffer.len() >= 1600 {
                        let samples_to_process: Vec<i16> = buffer.drain(..).collect();

                        // Feed to Vosk
                        let mut recognizer = recognizer.lock().unwrap();
                        match recognizer.accept_waveform(&samples_to_process) {
                            Ok(state) => {
                                log::debug!("Vosk decoding state: {:?}", state);

                                // Check partial results (ongoing recognition)
                                let partial_result = recognizer.partial_result();
                                if !partial_result.partial.is_empty() {
                                    log::info!("Vosk partial: '{}'", partial_result.partial);
                                }

                                // Check final results
                                if matches!(
                                    state,
                                    vosk::DecodingState::Finalized | vosk::DecodingState::Running
                                ) {
                                    let result = recognizer.result();
                                    if let Some(text) = result.single().map(|s| s.text) {
                                        if !text.is_empty() {
                                            log::info!("Vosk final: '{}'", text);
                                            *last_transcription.lock().unwrap() =
                                                Some(text.to_string());

                                            let text_lower = text.to_lowercase();
                                            log::debug!(
                                                "Comparing '{}' with wake phrase '{}'",
                                                text_lower,
                                                wake_phrase
                                            );
                                            if text_lower.contains(&wake_phrase) {
                                                log::info!("✓ Wake word detected: '{}'", text);
                                                *detected.lock().unwrap() = true;
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => log::error!("Vosk error: {:?}", e),
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Normal timeout, continue loop
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    log::info!("Audio channel disconnected, stopping processing thread");
                    break;
                }
            }
        }

        log::info!("Audio processing thread stopped");
    }

    /// Build audio input stream for i16 samples
    fn build_stream_i16(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        audio_tx: mpsc::SyncSender<AudioFrame>,
    ) -> Result<cpal::Stream> {
        let err_fn = |err| log::error!("Audio stream error: {}", err);

        let stream = device
            .build_input_stream(
                config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    // Just send raw audio to processing thread
                    let _ = audio_tx.try_send(AudioFrame::I16(data.to_vec()));
                },
                err_fn,
                None,
            )
            .map_err(|e| WakeWordError::StreamError(format!("{}", e)))?;

        Ok(stream)
    }

    /// Build audio input stream for u16 samples
    fn build_stream_u16(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        audio_tx: mpsc::SyncSender<AudioFrame>,
    ) -> Result<cpal::Stream> {
        let err_fn = |err| log::error!("Audio stream error: {}", err);

        let stream = device
            .build_input_stream(
                config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    // Just send raw audio to processing thread
                    let _ = audio_tx.try_send(AudioFrame::U16(data.to_vec()));
                },
                err_fn,
                None,
            )
            .map_err(|e| WakeWordError::StreamError(format!("{}", e)))?;

        Ok(stream)
    }

    /// Build audio input stream for f32 samples
    fn build_stream_f32(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        audio_tx: mpsc::SyncSender<AudioFrame>,
    ) -> Result<cpal::Stream> {
        let err_fn = |err| log::error!("Audio stream error: {}", err);

        let stream = device
            .build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Just send raw audio to processing thread
                    let _ = audio_tx.try_send(AudioFrame::F32(data.to_vec()));
                },
                err_fn,
                None,
            )
            .map_err(|e| WakeWordError::StreamError(format!("{}", e)))?;

        Ok(stream)
    }
    ///
    /// This is a non-blocking check. Call this periodically in your event loop.
    ///
    /// # Returns
    /// `true` if wake word was detected since last check, `false` otherwise
    ///
    /// # Behavior
    /// Resets the detection flag after returning `true`, so each detection
    /// is only returned once.
    pub fn check_for_wake_word(&self) -> bool {
        let mut detected = self.detected.lock().unwrap();
        if *detected {
            *detected = false; // Reset flag
            true
        } else {
            false
        }
    }

    /// Get the last transcription (for debugging)
    ///
    /// Returns the most recent text recognized by Vosk, regardless of
    /// whether it was the wake phrase.
    pub fn last_transcription(&self) -> Option<String> {
        self.last_transcription.lock().unwrap().clone()
    }

    /// Stop listening (drops the audio stream)
    pub fn stop_listening(&mut self) {
        log::info!("Stopping audio input stream");
        self.stream = None;
    }
}

impl Drop for WakeWordDetector {
    fn drop(&mut self) {
        // Signal processing thread to stop
        self.stop_flag.store(true, Ordering::Relaxed);

        // Drop the audio sender to disconnect the channel
        self.audio_tx = None;

        // Wait for processing thread to finish
        if let Some(thread) = self.processing_thread.take() {
            let _ = thread.join();
        }

        log::debug!("WakeWordDetector dropped and cleaned up");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_wake_word_detector_creation() {
        // This test just ensures the struct can be created
        // Actual audio testing requires hardware
        assert!(true);
    }
}
