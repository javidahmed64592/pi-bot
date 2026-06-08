//! PIR Sensor Task
//!
//! Monitors PIR sensor for motion detection and emits presence events.
//!
//! # Architecture
//! - Polls PIR sensor at regular intervals
//! - Emits Event::PresenceDetected on motion
//! - Emits Event::NoPresenceSince(duration) on timeout
//! - Emits Event::DeskPresenceDuration(minutes) at random intervals while present
//!   (using passive_observation_interval from config)
//! - Handles graceful shutdown

use anyhow::Result;
use bot_core::{Event, SystemConfig};
use sensors::PirSensorController;
use std::time::{Duration, Instant};
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
/// or presence timeout expires. While presence is continuously detected,
/// emits DeskPresenceDuration at random intervals (from passive_observation_interval)
/// for the controller to decide whether to initiate conversation.
/// Exits gracefully on shutdown signal.
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

    // Presence duration tracking
    let mut presence_start: Option<Instant> = None;
    let mut next_presence_emit: Option<Instant> = None;

    loop {
        tokio::select! {
            // Poll PIR sensor
            _ = interval.tick() => {
                if let Some(event) = pir.check_motion() {
                    log::debug!("[PIR Sensor Task] Event: {:?}", event);

                    match &event {
                        Event::PresenceDetected => {
                            // Start tracking continuous presence if not already
                            if presence_start.is_none() {
                                presence_start = Some(Instant::now());
                                // Schedule first emission at a random interval
                                let interval = config.behavior.random_observation_interval();
                                next_presence_emit = Some(Instant::now() + interval);
                                log::debug!(
                                    "[PIR Sensor Task] Presence tracking started, next emit in {:.0}s",
                                    interval.as_secs_f32()
                                );
                            }
                        }
                        Event::NoPresenceSince(_) => {
                            // Reset presence tracking
                            presence_start = None;
                            next_presence_emit = None;
                            log::debug!("[PIR Sensor Task] Presence tracking reset");
                        }
                        _ => {}
                    }

                    if let Err(e) = event_tx.send(event).await {
                        log::error!("[PIR Sensor Task] Failed to send event: {}", e);
                        break;
                    }
                }

                // Emit DeskPresenceDuration at random intervals while user is present
                if let (Some(start), Some(next_emit)) = (presence_start, next_presence_emit) {
                    if Instant::now() >= next_emit {
                        let minutes = start.elapsed().as_secs() / 60;
                        log::debug!(
                            "[PIR Sensor Task] Emitting DeskPresenceDuration: {} min",
                            minutes
                        );

                        if let Err(e) = event_tx.send(Event::DeskPresenceDuration(minutes as u32)).await {
                            log::error!("[PIR Sensor Task] Failed to send presence duration: {}", e);
                            break;
                        }

                        // Schedule next emission at a new random interval
                        let interval = config.behavior.random_observation_interval();
                        next_presence_emit = Some(Instant::now() + interval);
                        log::debug!(
                            "[PIR Sensor Task] Next presence emit in {:.0}s",
                            interval.as_secs_f32()
                        );
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
