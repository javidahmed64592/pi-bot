//! # TTS Test Binary
//!
//! Test binary for Piper TTS and speaker output.
//!
//! ## Usage
//!
//! ```bash
//! # Build with feature flag (requires Vosk library)
//! cargo build --bin tts-test --features actuators/tts-test
//!
//! # Run with default config
//! cargo run --bin tts-test --features actuators/tts-test
//!
//! # Test with specific text
//! cargo run --bin tts-test --features actuators/tts-test -- "Hello, I am Pi Bot"
//! ```
//!
//! ## What it tests
//!
//! 1. Piper TTS initialization
//! 2. Text-to-speech synthesis
//! 3. Audio playback through speaker
//! 4. End-to-end TTS pipeline
//!
//! ## Requirements
//!
//! - Piper TTS installed and in PATH
//! - Piper model downloaded (see config/config.yaml)
//! - Speaker/audio output device configured

use actuators::SpeakerController;
use anyhow::{Context, Result};
use audio_pipeline::PiperTts;
use bot_core::config::{load_config, SystemConfig};
use log::{error, info};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("=== Pi Bot TTS Test ===");

    // Load configuration
    let config = load_config("config/config.yaml").context("Failed to load configuration")?;

    // Get test text from command line or use default
    let args: Vec<String> = env::args().collect();
    let test_text = if args.len() > 1 {
        args[1..].join(" ")
    } else {
        "Hello, I am Pi Bot. Text to speech is working correctly.".to_string()
    };

    info!("Test text: '{}'", test_text);

    // Run TTS test
    match run_tts_test(&config, &test_text).await {
        Ok(_) => {
            info!("✓ TTS test completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("✗ TTS test failed: {}", e);
            Err(e)
        }
    }
}

async fn run_tts_test(config: &SystemConfig, text: &str) -> Result<()> {
    // Step 1: Initialize Piper TTS
    info!("Step 1: Initializing Piper TTS...");
    let mut tts = PiperTts::new(config.audio.piper.clone())
        .await
        .context("Failed to initialize Piper TTS")?;
    info!("  ✓ Piper TTS initialized with voice: {}", tts.voice());

    // Step 2: Synthesize speech
    info!("Step 2: Synthesizing speech...");
    let audio_data = tts
        .synthesize(text)
        .await
        .context("Failed to synthesize speech")?;
    info!("  ✓ Synthesized {} bytes of audio data", audio_data.len());

    // Step 3: Initialize speaker
    info!("Step 3: Initializing speaker...");
    let mut speaker = SpeakerController::new(&config.audio.speaker_device)
        .context("Failed to initialize speaker")?;
    info!(
        "  ✓ Speaker initialized on device: {}",
        speaker.device_name()
    );

    // Step 4: Play audio
    info!("Step 4: Playing audio...");
    speaker.play(&audio_data).context("Failed to play audio")?;
    info!("  ✓ Audio playback started");

    // Wait for playback to complete
    info!("Waiting for playback to complete...");
    speaker.wait_for_completion();
    info!("  ✓ Playback completed");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires config file and hardware
    async fn test_config_loading() {
        let result = load_config("config/config.yaml");
        assert!(result.is_ok());
    }
}
