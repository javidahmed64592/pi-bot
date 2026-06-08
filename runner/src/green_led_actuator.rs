//! Green LED Actuator Task
//!
//! Controls green status LEDs with pattern support (solid, breathing).
//!
//! # Architecture
//! - Consumes Command::SetGreenLeds from controller
//! - Executes patterns (solid, breathing, off)
//! - Controls both green LED channels
//! - Handles graceful shutdown

use actuators::StatusLedController;
use anyhow::Result;
use bot_core::{Command, StatusLedPattern, SystemConfig};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};

/// Run green LED actuator task
///
/// # Arguments
/// * `config` - System configuration with GPIO pin mapping
/// * `command_rx` - Channel to receive commands from controller
/// * `startup_tx` - Channel to notify runner that this component is ready
/// * `shutdown_rx` - Shutdown signal receiver
///
/// # Behavior
/// Listens for SetGreenLeds commands and controls both green status LEDs
/// with synchronized patterns. Exits gracefully on shutdown signal.
pub async fn run_green_led_actuator(
    config: &SystemConfig,
    mut command_rx: mpsc::Receiver<Command>,
    startup_tx: mpsc::Sender<(String, bool)>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    log::info!("[Green LED Actuator Task] Starting...");

    // Initialize both green LED controllers
    let mut green_led_1 = StatusLedController::new(config.gpio.led_pins.green_1, "Green LED 1")?;
    let mut green_led_2 = StatusLedController::new(config.gpio.led_pins.green_2, "Green LED 2")?;

    log::info!("[Green LED Actuator Task] Initialized");

    // Signal runner that this component is ready
    if let Err(e) = startup_tx.send(("green_led".to_string(), true)).await {
        log::warn!(
            "[Green LED Actuator Task] Failed to send startup signal: {}",
            e
        );
    }

    // Current pattern state
    let mut current_pattern = StatusLedPattern::Off;
    let mut pattern_frame = 0u32;

    // Pattern animation interval (50ms = 20 FPS)
    let mut pattern_interval = tokio::time::interval(Duration::from_millis(50));

    loop {
        tokio::select! {
            // Receive commands
            Some(command) = command_rx.recv() => {
                match command {
                    Command::SetGreenLeds(pattern) => {
                        log::debug!("[Green LED Actuator Task] SetGreenLeds: {:?}", pattern);
                        current_pattern = pattern;
                        pattern_frame = 0;
                    }

                    _ => {
                        // Ignore commands not relevant to green LEDs
                        log::trace!("[Green LED Actuator Task] Ignoring command: {:?}", command);
                    }
                }
            }

            // Pattern animation tick
            _ = pattern_interval.tick() => {
                let brightness = animate_status_pattern(current_pattern, pattern_frame);
                green_led_1.set_brightness(brightness);
                green_led_2.set_brightness(brightness);
                pattern_frame = pattern_frame.wrapping_add(1);
            }

            // Shutdown signal
            _ = shutdown_rx.recv() => {
                log::info!("[Green LED Actuator Task] Shutdown signal received");
                green_led_1.turn_off();
                green_led_2.turn_off();
                break;
            }
        }
    }

    log::info!("[Green LED Actuator Task] Stopped");
    Ok(())
}

/// Animate status LED pattern based on frame number
///
/// # Arguments
/// * `pattern` - Pattern type (Solid, Breathing, Flashing, Off)
/// * `frame` - Current animation frame
///
/// # Returns
/// Brightness level (0.0-1.0) for this frame
fn animate_status_pattern(pattern: StatusLedPattern, frame: u32) -> f32 {
    match pattern {
        StatusLedPattern::Solid => 1.0,

        StatusLedPattern::Breathing => {
            // Smooth sine wave (2 second cycle = 40 frames at 20 FPS)
            let cycle_frames = 40;
            let phase = (frame % cycle_frames) as f32 / cycle_frames as f32;
            (phase * 2.0 * std::f32::consts::PI).sin() * 0.5 + 0.5
        }

        StatusLedPattern::Flashing => {
            // Fast blink (0.5 second on/off = 10 frames at 20 FPS)
            let cycle_frames = 20;
            if (frame % cycle_frames) < 10 {
                1.0
            } else {
                0.0
            }
        }

        StatusLedPattern::Off => 0.0,
    }
}
