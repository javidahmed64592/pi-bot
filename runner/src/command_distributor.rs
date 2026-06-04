//! Command Distributor Task
//!
//! Routes commands from AI controller to appropriate actuator tasks.
//!
//! # Architecture
//! - Receives commands from single controller channel
//! - Distributes to appropriate actuator channels based on command type
//! - Handles graceful shutdown

use anyhow::Result;
use bot_core::Command;
use tokio::sync::{broadcast, mpsc};

/// Run command distributor task
///
/// # Arguments
/// * `controller_cmd_rx` - Channel receiving commands from AI controller
/// * `rgb_led_tx` - Channel to RGB LED actuator
/// * `speaker_tx` - Channel to speaker actuator
/// * `green_led_tx` - Channel to green LED actuator
/// * `red_led_tx` - Channel to red LED actuator
/// * `shutdown_rx` - Shutdown signal receiver
///
/// # Behavior
/// Routes each command to the appropriate actuator channel(s).
/// Some commands (like state-based commands) may be broadcast to multiple actuators.
/// Exits gracefully on shutdown signal.
#[allow(clippy::too_many_arguments)]
pub async fn run_command_distributor(
    mut controller_cmd_rx: mpsc::Receiver<Command>,
    rgb_led_tx: mpsc::Sender<Command>,
    speaker_tx: mpsc::Sender<Command>,
    green_led_tx: mpsc::Sender<Command>,
    red_led_tx: mpsc::Sender<Command>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    log::info!("[Command Distributor Task] Starting...");

    loop {
        tokio::select! {
            // Receive command from controller
            Some(command) = controller_cmd_rx.recv() => {
                // Route command to appropriate actuator(s)
                match &command {
                    // RGB LED commands
                    Command::SetColor(_) | Command::SetPattern { .. } | Command::LedOff => {
                        if let Err(e) = rgb_led_tx.send(command.clone()).await {
                            log::error!("[Command Distributor] Failed to send to RGB LED: {}", e);
                        }
                    }

                    // Speaker commands
                    Command::Speak { .. } | Command::StopSpeaking => {
                        if let Err(e) = speaker_tx.send(command.clone()).await {
                            log::error!("[Command Distributor] Failed to send to speaker: {}", e);
                        }
                    }

                    // Green LED commands
                    Command::SetGreenLeds(_) => {
                        if let Err(e) = green_led_tx.send(command.clone()).await {
                            log::error!("[Command Distributor] Failed to send to green LEDs: {}", e);
                        }
                    }

                    // Red LED commands
                    Command::SetRedLeds(_) => {
                        if let Err(e) = red_led_tx.send(command.clone()).await {
                            log::error!("[Command Distributor] Failed to send to red LEDs: {}", e);
                        }
                    }

                    // System state commands (not sent to actuators, handled by controller)
                    Command::LockBot
                    | Command::UnlockBot
                    | Command::StartListening
                    | Command::StopListening
                    | Command::EnterConversationState(_)
                    | Command::SetLightingMode(_) => {
                        // These are informational or handled elsewhere
                        log::trace!("[Command Distributor] System command (not routed): {:?}", command);
                    }
                }
            }

            // Shutdown signal
            _ = shutdown_rx.recv() => {
                log::info!("[Command Distributor Task] Shutdown signal received");
                break;
            }
        }
    }

    log::info!("[Command Distributor Task] Stopped");
    Ok(())
}
