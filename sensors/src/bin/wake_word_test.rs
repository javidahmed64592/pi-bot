//! Wake Word Detection Test Binary
//!
//! Test wake word detection by listening for "hey bot" via USB microphone.
//! This binary continuously monitors audio input and logs when the wake word is detected.
//!
//! # Usage
//! ```bash
//! # Build with feature flag (requires Vosk library)
//! cargo run --bin wake_word_test --features sensors/audio
//! # Say "hey bot" and verify detection
//! ```
//!
//! # Requirements
//! - USB microphone connected and configured
//! - Vosk model downloaded to models/vosk/
//! - ALSA audio system (Linux) or equivalent

use anyhow::Result;
use bot_core::load_config;
use log::info;
use sensors::AudioController;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!("=== Wake Word Detection Test ===");
    info!("Testing Vosk wake word detection with USB microphone");
    info!("Say 'hey bot' to trigger detection");
    info!("Press Ctrl+C to exit\n");

    // Load configuration
    let config = load_config("config/config.yaml")?;

    info!("Configuration:");
    info!("  Model: {}", config.audio.vosk.model_path);
    info!("  Wake phrase: '{}'", config.audio.vosk.wake_phrase);
    info!("  Sample rate: {} Hz", config.audio.sample_rate);
    info!("  Microphone: {}\n", config.audio.microphone_device);

    // Initialize audio controller
    let mut audio = AudioController::new(&config, "WakeWordTest")?;

    // Start listening
    info!("Starting audio input stream...");
    audio.start()?;
    info!("Listening for wake word...\n");

    // Poll for wake word detection
    let mut detection_count = 0;
    loop {
        if let Some(event) = audio.poll() {
            detection_count += 1;
            info!("╔════════════════════════════════════════╗");
            info!("║ WAKE WORD DETECTED!                    ║");
            info!("╠════════════════════════════════════════╣");
            info!("║ Event: {:?}", event);
            info!("║ Detection #{}", detection_count);
            if let Some(transcript) = audio.last_transcription() {
                info!("║ Transcript: '{}'", transcript);
            }
            info!("╚════════════════════════════════════════╝\n");
        }

        // Poll every 100ms
        sleep(Duration::from_millis(100)).await;
    }
}
