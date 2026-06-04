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

    // Phase 1.12 - Main system integration
    // Components ready:
    // - PIR sensor with timeout detection
    // - RGB LED with pattern support
    // - Status LEDs with PWM brightness
    // - Audio pipeline (wake word, STT, TTS)
    // - LLM service with Ollama
    // - Memory service with session persistence
    //
    // Next: Wire all components via channels and spawn tasks

    log::info!("System ready. Awaiting shutdown signal...");

    // Placeholder: Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    log::info!("Shutdown signal received, exiting...");

    Ok(())
}
