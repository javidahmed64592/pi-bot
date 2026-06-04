//! RGB LED Actuator Task
//!
//! Controls RGB LED with color and pattern commands.
//!
//! # Architecture
//! - Consumes Command::SetColor and Command::SetPattern from controller
//! - Executes patterns (breathing, pulse, solid, etc.)
//! - Handles graceful shutdown

use actuators::RgbLedController;
use anyhow::Result;
use bot_core::{Command, LedPattern, RgbColor, SystemConfig};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};

/// Run RGB LED actuator task
///
/// # Arguments
/// * `config` - System configuration with GPIO pin mapping
/// * `command_rx` - Channel to receive commands from controller
/// * `shutdown_rx` - Shutdown signal receiver
///
/// # Behavior
/// Listens for SetColor and SetPattern commands and controls the RGB LED.
/// For patterns, runs animation loops. Exits gracefully on shutdown signal.
pub async fn run_rgb_led_actuator(
    config: &SystemConfig,
    mut command_rx: mpsc::Receiver<Command>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    log::info!("[RGB LED Actuator Task] Starting...");

    // Initialize RGB LED controller
    let mut rgb_led = RgbLedController::new(
        config.gpio.rgb_pins.red,
        config.gpio.rgb_pins.green,
        config.gpio.rgb_pins.blue,
        "RGB LED",
    )?;

    log::info!("[RGB LED Actuator Task] Initialized");

    // Current pattern state
    let mut current_pattern: Option<(LedPattern, Vec<RgbColor>)> = None;
    let mut pattern_frame = 0u32;

    // Pattern animation interval (50ms = 20 FPS)
    let mut pattern_interval = tokio::time::interval(Duration::from_millis(50));

    loop {
        tokio::select! {
            // Receive commands
            Some(command) = command_rx.recv() => {
                match command {
                    Command::SetColor(color) => {
                        log::debug!("[RGB LED Actuator Task] SetColor: {:?}", color);
                        current_pattern = None; // Stop any running pattern
                        rgb_led.set_color(color);
                    }

                    Command::SetPattern { pattern, colors } => {
                        log::debug!("[RGB LED Actuator Task] SetPattern: {:?} with {} colors", pattern, colors.len());
                        current_pattern = Some((pattern, colors));
                        pattern_frame = 0;
                    }

                    Command::LedOff => {
                        log::debug!("[RGB LED Actuator Task] LedOff");
                        current_pattern = None;
                        rgb_led.turn_off();
                    }

                    _ => {
                        // Ignore commands not relevant to RGB LED
                        log::trace!("[RGB LED Actuator Task] Ignoring command: {:?}", command);
                    }
                }
            }

            // Pattern animation tick
            _ = pattern_interval.tick() => {
                if let Some((pattern, ref colors)) = current_pattern {
                    if !colors.is_empty() {
                        let color = animate_pattern(pattern, colors, pattern_frame);
                        rgb_led.set_color(color);
                        pattern_frame = pattern_frame.wrapping_add(1);
                    }
                }
            }

            // Shutdown signal
            _ = shutdown_rx.recv() => {
                log::info!("[RGB LED Actuator Task] Shutdown signal received");
                rgb_led.turn_off();
                break;
            }
        }
    }

    log::info!("[RGB LED Actuator Task] Stopped");
    Ok(())
}

/// Animate LED pattern based on frame number
///
/// # Arguments
/// * `pattern` - Pattern type (Breathing, Pulse, Solid, etc.)
/// * `colors` - Colors to use in pattern
/// * `frame` - Current animation frame
///
/// # Returns
/// Color to display for this frame
fn animate_pattern(pattern: LedPattern, colors: &[RgbColor], frame: u32) -> RgbColor {
    match pattern {
        LedPattern::Solid => {
            // Static color (first color in list)
            colors[0]
        }

        LedPattern::Breathing => {
            // Smooth sine wave (2 second cycle = 40 frames at 20 FPS)
            let cycle_frames = 40;
            let phase = (frame % cycle_frames) as f32 / cycle_frames as f32;
            let brightness = (phase * 2.0 * std::f32::consts::PI).sin() * 0.5 + 0.5;
            colors[0].scale(brightness)
        }

        LedPattern::Pulse => {
            // Quick pulse (1 second cycle = 20 frames at 20 FPS)
            let cycle_frames = 20;
            let phase = (frame % cycle_frames) as f32 / cycle_frames as f32;
            let brightness = if phase < 0.3 {
                // Quick rise
                phase / 0.3
            } else {
                // Slow fade
                1.0 - ((phase - 0.3) / 0.7)
            };
            colors[0].scale(brightness)
        }

        LedPattern::Gradient => {
            // Transition between colors (3 second cycle = 60 frames at 20 FPS)
            if colors.len() < 2 {
                return colors[0];
            }
            let cycle_frames = 60;
            let phase = (frame % cycle_frames) as f32 / cycle_frames as f32;
            let segment_count = colors.len() - 1;
            let segment = (phase * segment_count as f32) as usize;
            let segment_phase = (phase * segment_count as f32) - segment as f32;

            let from = colors[segment];
            let to = colors[(segment + 1) % colors.len()];
            from.lerp(&to, segment_phase)
        }

        LedPattern::Rainbow => {
            // Full rainbow cycle (4 second cycle = 80 frames at 20 FPS)
            let cycle_frames = 80;
            let phase = (frame % cycle_frames) as f32 / cycle_frames as f32;
            RgbColor::from_hsv(phase * 360.0, 1.0, 1.0)
        }

        LedPattern::ColorCycle => {
            // Cycle through provided colors (1 second per color = 20 frames)
            let frames_per_color = 20;
            let color_index = ((frame / frames_per_color) as usize) % colors.len();
            colors[color_index]
        }
    }
}
