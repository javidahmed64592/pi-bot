//! PIR Sensor Task
//!
//! Monitors PIR sensor for motion detection and emits presence events.
//!
//! # Architecture
//! - Polls PIR sensor at regular intervals
//! - Emits Event::PresenceDetected on motion
//! - Emits Event::NoPresenceSince(duration) on timeout
//! - Handles graceful shutdown

use anyhow::Result;
use bot_core::{Event, SystemConfig};
use sensors::PirSensorController;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};

/// Run PIR sensor task
///
/// # Arguments
/// * `config` - System configuration with GPIO pin mapping
/// * `event_tx` - Channel to send events to controller
/// * `startup_tx` - Channel to notify runner that this component is ready
/// * `shutdown_rx` - Shutdown signal receiver
///
/// # Behavior
/// Polls PIR sensor every 100ms and sends events when motion detected
/// or presence timeout expires. Exits gracefully on shutdown signal.
pub async fn run_pir_sensor(
    config: &SystemConfig,
    event_tx: mpsc::Sender<Event>,
    startup_tx: mpsc::Sender<(String, bool)>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    log::info!("[PIR Sensor Task] Starting...");

    // Initialize PIR sensor controller
    let timeout_duration = Duration::from_secs(config.behavior.idle_timeout);
    let mut pir = PirSensorController::new(config.gpio.pir_pin, "PIR Sensor", timeout_duration)?;

    log::info!("[PIR Sensor Task] Initialized, polling every 100ms");

    // Signal runner that this component is ready
    if let Err(e) = startup_tx.send(("pir".to_string(), true)).await {
        log::warn!("[PIR Sensor Task] Failed to send startup signal: {}", e);
    }

    // Polling interval
    let poll_interval = Duration::from_millis(100);
    let mut interval = tokio::time::interval(poll_interval);

    loop {
        tokio::select! {
            // Poll PIR sensor
            _ = interval.tick() => {
                if let Some(event) = pir.check_motion() {
                    log::debug!("[PIR Sensor Task] Event: {:?}", event);

                    if let Err(e) = event_tx.send(event).await {
                        log::error!("[PIR Sensor Task] Failed to send event: {}", e);
                        break;
                    }
                }
            }

            // Shutdown signal
            _ = shutdown_rx.recv() => {
                log::info!("[PIR Sensor Task] Shutdown signal received");
                break;
            }
        }
    }

    log::info!("[PIR Sensor Task] Stopped");
    Ok(())
}
