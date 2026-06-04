//! Red LED Actuator Task
//!
//! Controls red status LEDs with pattern support (breathing, flashing).
//!
//! # Architecture
//! - Consumes Command::SetRedLeds from controller
//! - Executes patterns (breathing, flashing, off)
//! - Controls both red LED channels
//! - Handles graceful shutdown

use actuators::StatusLedController;
use anyhow::Result;
use bot_core::{Command, StatusLedPattern, SystemConfig};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};

/// Run red LED actuator task
///
/// # Arguments
/// * `config` - System configuration with GPIO pin mapping
/// * `command_rx` - Channel to receive commands from controller
/// * `shutdown_rx` - Shutdown signal receiver
///
/// # Behavior
/// Listens for SetRedLeds commands and controls both red status LEDs
/// with synchronized patterns. Exits gracefully on shutdown signal.
pub async fn run_red_led_actuator(
    config: &SystemConfig,
    mut command_rx: mpsc::Receiver<Command>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    log::info!("[Red LED Actuator Task] Starting...");

    // Initialize both red LED controllers
    let mut red_led_1 = StatusLedController::new(config.gpio.led_pins.red_1, "Red LED 1")?;
    let mut red_led_2 = StatusLedController::new(config.gpio.led_pins.red_2, "Red LED 2")?;

    log::info!("[Red LED Actuator Task] Initialized");

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
                    Command::SetRedLeds(pattern) => {
                        log::debug!("[Red LED Actuator Task] SetRedLeds: {:?}", pattern);
                        current_pattern = pattern;
                        pattern_frame = 0;
                    }

                    _ => {
                        // Ignore commands not relevant to red LEDs
                        log::trace!("[Red LED Actuator Task] Ignoring command: {:?}", command);
                    }
                }
            }

            // Pattern animation tick
            _ = pattern_interval.tick() => {
                let brightness = animate_status_pattern(current_pattern, pattern_frame);
                red_led_1.set_brightness(brightness);
                red_led_2.set_brightness(brightness);
                pattern_frame = pattern_frame.wrapping_add(1);
            }

            // Shutdown signal
            _ = shutdown_rx.recv() => {
                log::info!("[Red LED Actuator Task] Shutdown signal received");
                red_led_1.turn_off();
                red_led_2.turn_off();
                break;
            }
        }
    }

    log::info!("[Red LED Actuator Task] Stopped");
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
