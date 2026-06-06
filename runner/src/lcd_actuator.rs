//! LCD Actuator Task
//!
//! Handles LCD display operations for 16x2 I2C LCD displays.
//!
//! # Architecture
//! - Consumes LCD-related commands from controller
//! - Controls LCD via LcdController hardware interface
//! - Handles display text, backlight, and display on/off control
//! - Handles graceful shutdown

use actuators::LcdController;
use anyhow::Result;
use bot_core::{Command, SystemConfig};
use tokio::sync::{broadcast, mpsc};

/// Run LCD actuator task
///
/// # Arguments
/// * `config` - System configuration with LCD I2C address
/// * `command_rx` - Channel to receive commands from controller
/// * `shutdown_rx` - Shutdown signal receiver
///
/// # Behavior
/// Listens for LCD commands (DisplayText, ClearDisplay, SetBacklight).
/// Controls the LCD hardware via LcdController.
/// Gracefully handles errors and shutdown.
pub async fn run_lcd_actuator(
    config: &SystemConfig,
    mut command_rx: mpsc::Receiver<Command>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    log::info!("[LCD Actuator Task] Starting...");

    // Initialize LCD controller
    let mut lcd = match LcdController::new(config.gpio.lcd_i2c_address, "LCD Display") {
        Ok(controller) => controller,
        Err(e) => {
            log::error!("[LCD Actuator Task] Failed to initialize LCD: {}", e);
            log::warn!("[LCD Actuator Task] Running in degraded mode (LCD unavailable)");
            // Wait for shutdown signal (non-critical component, allow system to continue)
            let _ = shutdown_rx.recv().await;
            return Ok(());
        }
    };

    log::info!(
        "[LCD Actuator Task] Initialized (I2C: 0x{:02X})",
        config.gpio.lcd_i2c_address
    );

    loop {
        tokio::select! {
            // Receive commands
            Some(command) = command_rx.recv() => {
                match command {
                    Command::DisplayText { line1, line2, duration_ms } => {
                        log::debug!("[LCD Actuator Task] DisplayText: '{}' / '{}' (duration: {:?}ms)", line1, line2, duration_ms);

                        // If duration is specified, use the timed display method
                        if let Some(duration) = duration_ms {
                            let duration = std::time::Duration::from_millis(duration);
                            if let Err(e) = lcd.display_with_duration(&line1, &line2, duration).await {
                                log::error!("[LCD Actuator Task] Failed to display with duration: {}", e);
                            }
                        } else {
                            // Original behavior: just write the lines without auto-clear
                            // Write line 1
                            if let Err(e) = lcd.write_line(0, &line1) {
                                log::error!("[LCD Actuator Task] Failed to write line 1: {}", e);
                            }

                            // Write line 2
                            if let Err(e) = lcd.write_line(1, &line2) {
                                log::error!("[LCD Actuator Task] Failed to write line 2: {}", e);
                            }
                        }
                    }

                    Command::ClearDisplay => {
                        log::debug!("[LCD Actuator Task] ClearDisplay");
                        if let Err(e) = lcd.clear() {
                            log::error!("[LCD Actuator Task] Failed to clear display: {}", e);
                        }
                    }

                    Command::SetBacklight { on } => {
                        log::debug!("[LCD Actuator Task] SetBacklight: {}", on);
                        let result = if on {
                            lcd.backlight_on()
                        } else {
                            lcd.backlight_off()
                        };

                        if let Err(e) = result {
                            log::error!("[LCD Actuator Task] Failed to set backlight: {}", e);
                        }
                    }

                    _ => {
                        // Ignore commands not relevant to LCD
                        log::trace!("[LCD Actuator Task] Ignoring command: {:?}", command);
                    }
                }
            }

            // Shutdown signal
            _ = shutdown_rx.recv() => {
                log::info!("[LCD Actuator Task] Shutting down...");

                // Clear display on shutdown
                if let Err(e) = lcd.clear() {
                    log::error!("[LCD Actuator Task] Failed to clear display on shutdown: {}", e);
                }

                // Turn off backlight on shutdown
                if let Err(e) = lcd.backlight_off() {
                    log::error!("[LCD Actuator Task] Failed to turn off backlight on shutdown: {}", e);
                }

                break;
            }
        }
    }

    log::info!("[LCD Actuator Task] Shutdown complete");
    Ok(())
}
