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

/// Audio sensor commands (for controlling detection)
#[derive(Debug, Clone)]
pub enum AudioSensorCommand {
    EnableDetection,
    DisableDetection,
}

/// Run audio sensor task in a dedicated thread
///
/// # Arguments
/// * `config` - System configuration with audio settings
/// * `event_tx` - Channel to send events to controller
/// * `cmd_rx` - Channel to receive commands (for enabling/disabling detection)
/// * `shutdown_rx` - Shutdown signal receiver
/// * `ready_tx` - Channel to notify runner that system is ready
///
/// # Behavior
/// Continuously polls AudioController for wake word and speech events.
/// Sends events to the controller. Exits gracefully on shutdown signal.
pub async fn run_audio_sensor(
    config: &SystemConfig,
    event_tx: mpsc::Sender<Event>,
    mut cmd_rx: mpsc::Receiver<AudioSensorCommand>,
    mut shutdown_rx: broadcast::Receiver<()>,
    startup_tx: mpsc::Sender<(String, bool)>,
) -> Result<()> {
    log::info!("[Audio Sensor Task] Starting...");

    // Atomic flag for shutdown signaling to blocking thread
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();

    // Create crossbeam channel for sending commands to blocking thread
    let (cmd_tx_crossbeam, cmd_rx_crossbeam) = crossbeam_channel::unbounded::<AudioSensorCommand>();

    // Spawn blocking thread for audio processing
    let config_clone = config.clone();
    let event_tx_clone = event_tx.clone();
    let audio_thread = thread::spawn(move || {
        let config = config_clone;
        let event_tx = event_tx_clone;
        let cmd_rx = cmd_rx_crossbeam;

        // Initialize audio controller
        let mut audio = match AudioController::new(&config, "Audio Sensor") {
            Ok(a) => a,
            Err(e) => {
                log::error!("[Audio Sensor Task] Failed to initialize: {}", e);
                // Signal failure so runner can proceed (degraded mode)
                if startup_tx.blocking_send(("audio".to_string(), false)).is_err() {
                    log::error!("[Audio Sensor Task] Failed to send startup failure signal");
                }
                return;
            }
        };

        // Start listening for wake word
        if let Err(e) = audio.start() {
            log::error!("[Audio Sensor Task] Failed to start: {}", e);
            if startup_tx.blocking_send(("audio".to_string(), false)).is_err() {
                log::error!("[Audio Sensor Task] Failed to send startup failure signal");
            }
            return;
        }

        log::info!(
            "[Audio Sensor Task] Listening for wake word: '{}'",
            config.audio.vosk.wake_phrase
        );

        // Signal runner that Vosk has finished loading and the sensor is ready.
        // The runner forwards this as Event::ComponentReady { component: "audio" }
        // to the controller, which transitions the LEDs from loading to ready.
        if startup_tx.blocking_send(("audio".to_string(), true)).is_err() {
            log::error!("[Audio Sensor Task] Failed to send startup signal");
        } else {
            log::info!("[Audio Sensor Task] Vosk model loaded — audio sensor ready");
        }

        // Poll interval (check every 50ms for low latency)
        let poll_interval = Duration::from_millis(50);

        loop {
            // Check shutdown flag
            if shutdown_flag_clone.load(Ordering::Relaxed) {
                log::info!("[Audio Sensor Task] Shutdown flag set");
                break;
            }

            // Check for commands (non-blocking)
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    AudioSensorCommand::EnableDetection => {
                        log::info!("[Audio Sensor Task] Enabling audio detection");
                        audio.enable_detection();
                    }
                    AudioSensorCommand::DisableDetection => {
                        log::info!("[Audio Sensor Task] Disabling audio detection");
                        audio.disable_detection();
                    }
                }
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

    // Forward commands from async tokio channel to crossbeam channel
    let _cmd_forwarder = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    if cmd_tx_crossbeam.send(cmd).is_err() {
                        log::error!("[Audio Sensor Task] Failed to forward command (thread stopped)");
                        break;
                    }
                }
                else => break,
            }
        }
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
