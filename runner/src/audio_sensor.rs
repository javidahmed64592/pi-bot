//! Audio Sensor Task
//!
//! Manages wake word detection and speech-to-text transcription.
//!
//! # Architecture
//! - Polls AudioController for wake word and speech events
//! - Emits Event::WakeWordDetected when "hey bot" is heard
//! - Emits Event::SpeechCaptured(String) with transcribed text
//! - Handles graceful shutdown
//!
//! Note: Runs in a dedicated thread (not tokio async) because AudioController
//! contains Vosk bindings which are not Send.

use anyhow::Result;
use bot_core::{Event, SystemConfig};
use sensors::AudioController;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};

/// Run audio sensor task in a dedicated thread
///
/// # Arguments
/// * `config` - System configuration with audio settings
/// * `event_tx` - Channel to send events to controller
/// * `shutdown_rx` - Shutdown signal receiver
///
/// # Behavior
/// Continuously polls AudioController for wake word and speech events.
/// Sends events to the controller. Exits gracefully on shutdown signal.
pub async fn run_audio_sensor(
    config: &SystemConfig,
    event_tx: mpsc::Sender<Event>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    log::info!("[Audio Sensor Task] Starting...");

    // Atomic flag for shutdown signaling to blocking thread
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();

    // Spawn blocking thread for audio processing
    let config_clone = config.clone();
    let audio_thread = thread::spawn(move || {
        let config = config_clone;

        // Initialize audio controller
        let mut audio = match AudioController::new(&config, "Audio Sensor") {
            Ok(a) => a,
            Err(e) => {
                log::error!("[Audio Sensor Task] Failed to initialize: {}", e);
                return;
            }
        };

        // Start listening for wake word
        if let Err(e) = audio.start() {
            log::error!("[Audio Sensor Task] Failed to start: {}", e);
            return;
        }

        log::info!(
            "[Audio Sensor Task] Listening for wake word: '{}'",
            config.audio.vosk.wake_phrase
        );

        // Poll interval (check every 50ms for low latency)
        let poll_interval = Duration::from_millis(50);

        loop {
            // Check shutdown flag
            if shutdown_flag_clone.load(Ordering::Relaxed) {
                log::info!("[Audio Sensor Task] Shutdown flag set");
                break;
            }

            // Poll audio controller
            if let Some(event) = audio.poll() {
                log::debug!("[Audio Sensor Task] Event: {:?}", event);

                // Send event (blocking send, okay since we're in dedicated thread)
                if event_tx.blocking_send(event).is_err() {
                    log::error!("[Audio Sensor Task] Failed to send event (channel closed)");
                    break;
                }
            }

            // Sleep to avoid busy-waiting
            thread::sleep(poll_interval);
        }

        log::info!("[Audio Sensor Task] Audio thread stopped");
    });

    // Wait for shutdown signal in async context
    let _ = shutdown_rx.recv().await;
    log::info!("[Audio Sensor Task] Shutdown signal received");

    // Set shutdown flag for blocking thread
    shutdown_flag.store(true, Ordering::Relaxed);

    // Wait for audio thread to complete (with timeout)
    let timeout = Duration::from_secs(2);
    let start = std::time::Instant::now();
    while !audio_thread.is_finished() && start.elapsed() < timeout {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    log::info!("[Audio Sensor Task] Stopped");
    Ok(())
}
