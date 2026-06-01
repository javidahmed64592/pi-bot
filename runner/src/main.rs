//! # runner
//!
//! Main orchestration binary for the Pi Bot companion system.
//!
//! Responsibilities:
//! - Load configuration from config.yaml
//! - Initialize tokio channels for events/commands
//! - Spawn sensor tasks
//! - Spawn actuator tasks
//! - Spawn controller task
//! - Spawn audio pipeline tasks
//! - Handle graceful shutdown

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    log::info!("Pi Bot companion system starting...");

    // TODO: Phase 1.4 - Load config
    // TODO: Phase 1.4 - Create channels
    // TODO: Phase 1.4 - Spawn controller task
    // TODO: Phase 1.5 - Spawn PIR sensor task
    // TODO: Phase 1.6 - Spawn RGB LED task
    // TODO: Phase 1.6 - Spawn status LED tasks
    // TODO: Phase 1.11 - Spawn wake word task
    // TODO: Phase 1.11 - Spawn STT task
    // TODO: Phase 1.11 - Spawn TTS task
    // TODO: Phase 1.12 - Implement graceful shutdown

    log::info!("All tasks spawned, waiting for shutdown signal...");

    // Placeholder: Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    log::info!("Shutdown signal received, exiting...");

    Ok(())
}
