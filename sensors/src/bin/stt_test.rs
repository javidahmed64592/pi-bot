//! Speech-to-Text Test Binary
//!
//! Test the full wake word → speech capture → transcription flow.
//! This binary listens for the wake word ("hey"), then captures and transcribes
//! the following speech.
//!
//! # Usage
//! ```bash
//! # Build with feature flag (requires Vosk library)
//! cargo run --bin stt_test --features sensors/audio
//! # Say "hey" (wake word)
//! # Then speak your message
//! # Wait for transcription result
//! ```
//!
//! # Flow
//! 1. Listens for wake word ("hey")
//! 2. When detected, switches to speech capture mode
//! 3. Captures speech until silence detected or timeout
//! 4. Transcribes captured speech
//! 5. Returns to wake word mode
//!
//! # Requirements
//! - USB microphone connected and configured
//! - Vosk model downloaded to models/vosk/
//! - ALSA audio system (Linux) or equivalent

use anyhow::Result;
use bot_core::{load_config, Event};
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

    info!("=== Speech-to-Text Test ===");
    info!("Testing Vosk STT with wake word activation");
    info!("1. Say '{}' to activate", "hey");
    info!("2. Speak your message");
    info!("3. Wait for transcription");
    info!("Press Ctrl+C to exit\n");

    // Load configuration
    let config = load_config("config/config.yaml")?;

    info!("Configuration:");
    info!("  Model: {}", config.audio.vosk.model_path);
    info!("  Wake phrase: '{}'", config.audio.vosk.wake_phrase);
    info!("  Sample rate: {} Hz", config.audio.sample_rate);
    info!(
        "  Capture timeout: {:.1}s",
        config.audio.vosk.stt.capture_timeout
    );
    info!(
        "  Silence threshold: {:.1}s",
        config.audio.vosk.stt.silence_threshold
    );
    info!(
        "  Min speech duration: {:.1}s",
        config.audio.vosk.stt.min_speech_duration
    );
    info!("  Microphone: {}\n", config.audio.microphone_device);

    // Initialize audio controller
    let mut audio = AudioController::new(&config, "SttTest")?;

    // Start listening
    info!("Starting audio input stream...");
    audio.start()?;
    info!(
        "Listening for wake word '{}' ...\n",
        config.audio.vosk.wake_phrase
    );

    // Poll for events
    let mut wake_count = 0;
    let mut speech_count = 0;
    let mut is_capturing = false;

    loop {
        if let Some(event) = audio.poll() {
            match event {
                Event::WakeWordDetected => {
                    wake_count += 1;
                    is_capturing = true;
                    info!("╔═══════════════════════════════════════════════╗");
                    info!(
                        "║ WAKE WORD DETECTED! (#{})                      ║",
                        wake_count
                    );
                    info!("╠═══════════════════════════════════════════════╣");
                    info!("║ Now capturing speech...                       ║");
                    info!("║ Speak your message now!                       ║");
                    info!("╚═══════════════════════════════════════════════╝\n");
                }
                Event::SpeechCaptured(transcript) => {
                    speech_count += 1;
                    is_capturing = false;
                    info!("╔═══════════════════════════════════════════════╗");
                    info!(
                        "║ SPEECH CAPTURED! (#{})                         ║",
                        speech_count
                    );
                    info!("╠═══════════════════════════════════════════════╣");
                    info!("║ Transcript:");
                    info!("║ \"{}\"", transcript);
                    info!("╠═══════════════════════════════════════════════╣");
                    info!("║ Back to listening for wake word...            ║");
                    info!("╚═══════════════════════════════════════════════╝\n");
                }
                _ => {
                    // Ignore other events
                }
            }
        }

        // Show status indicator
        if is_capturing {
            static mut CAPTURE_DOTS: usize = 0;
            unsafe {
                CAPTURE_DOTS = (CAPTURE_DOTS + 1) % 4;
                let dots = ".".repeat(CAPTURE_DOTS);
                print!("\r[Capturing speech{}   ]", dots);
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }

        // Poll every 100ms
        sleep(Duration::from_millis(100)).await;
    }
}
