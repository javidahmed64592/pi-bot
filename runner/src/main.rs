//! # runner
//!
//! Main orchestration binary for the Pi Bot companion system.
//!
//! Responsibilities:
//! - Load configuration from config.yaml
//! - Initialize tokio channels for events/commands
//! - Spawn sensor tasks (PIR, audio)
//! - Spawn actuator tasks (RGB LED, speaker, green LEDs, red LEDs)
//! - Spawn AI controller task
//! - Handle graceful shutdown

mod audio_sensor;
mod command_distributor;
mod green_led_actuator;
mod pir_sensor;
mod red_led_actuator;
mod rgb_led_actuator;
mod speaker_actuator;

use anyhow::Result;
use bot_core::{load_config, Command, Event, StatusLedPattern};
use tokio::sync::{broadcast, mpsc};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("==============================================================");
    log::info!("           Pi Bot Companion System v0.1.0                  ");
    log::info!("==============================================================");

    // ========================================================================
    // 1. Load Configuration
    // ========================================================================

    log::info!("Loading configuration from config/config.yaml...");
    let config = load_config("config/config.yaml")?;
    log::info!("Configuration loaded successfully");

    // ========================================================================
    // 2. Initialize Channels
    // ========================================================================

    log::info!("Initializing communication channels...");

    // Event channel: Sensors → Controller
    let (event_tx, event_rx) = mpsc::channel::<Event>(32);

    // Controller command channel: Controller → Command Distributor
    let (controller_cmd_tx, controller_cmd_rx) = mpsc::channel::<Command>(32);

    // Actuator command channels: Command Distributor → Actuators
    let (rgb_led_tx, rgb_led_rx) = mpsc::channel::<Command>(32);
    let (speaker_tx, speaker_rx) = mpsc::channel::<Command>(32);
    let (green_led_tx, green_led_rx) = mpsc::channel::<Command>(32);
    let (red_led_tx, red_led_rx) = mpsc::channel::<Command>(32);

    // Audio sensor command channel: Command Distributor → Audio Sensor
    let (audio_cmd_tx, audio_cmd_rx) = mpsc::channel::<audio_sensor::AudioSensorCommand>(32);

    // Shutdown channel (broadcast to all tasks)
    let (shutdown_tx, _) = broadcast::channel::<()>(16);

    log::info!("Channels initialized");

    // ========================================================================
    // 3. Spawn LED Actuator Tasks First (for loading indicator)
    // ========================================================================

    log::info!("Spawning LED actuator tasks...");

    // RGB LED Actuator Task
    let rgb_led_task = {
        let config = config.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            if let Err(e) =
                rgb_led_actuator::run_rgb_led_actuator(&config, rgb_led_rx, shutdown_rx).await
            {
                log::error!("RGB LED actuator task failed: {}", e);
            }
        })
    };

    // Green LED Actuator Task
    let green_led_task = {
        let config = config.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            if let Err(e) =
                green_led_actuator::run_green_led_actuator(&config, green_led_rx, shutdown_rx).await
            {
                log::error!("Green LED actuator task failed: {}", e);
            }
        })
    };

    // Red LED Actuator Task
    let red_led_task = {
        let config = config.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            if let Err(e) =
                red_led_actuator::run_red_led_actuator(&config, red_led_rx, shutdown_rx).await
            {
                log::error!("Red LED actuator task failed: {}", e);
            }
        })
    };

    log::info!("LED actuator tasks spawned");

    // Give LED actuators a moment to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // ========================================================================
    // 4. Spawn Sensor Tasks (Heavy: Vosk model loading)
    // ========================================================================

    log::info!("Spawning sensor tasks...");

    // PIR Sensor Task
    let pir_task = {
        let config = config.clone();
        let event_tx = event_tx.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            if let Err(e) = pir_sensor::run_pir_sensor(&config, event_tx, shutdown_rx).await {
                log::error!("PIR sensor task failed: {}", e);
            }
        })
    };

    // Audio Sensor Task (wake word + STT)
    let audio_task = {
        let config = config.clone();
        let event_tx = event_tx.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            if let Err(e) =
                audio_sensor::run_audio_sensor(&config, event_tx, audio_cmd_rx, shutdown_rx).await
            {
                log::error!("Audio sensor task failed: {}", e);
            }
        })
    };

    log::info!("Sensor tasks spawned");

    // ========================================================================
    // 5. Spawn Speaker Actuator Task
    // ========================================================================

    log::info!("Spawning speaker actuator task...");

    // Speaker Actuator Task
    let speaker_task = {
        let config = config.clone();
        let event_tx = event_tx.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            if let Err(e) =
                speaker_actuator::run_speaker_actuator(&config, speaker_rx, event_tx, shutdown_rx)
                    .await
            {
                log::error!("Speaker actuator task failed: {}", e);
            }
        })
    };

    log::info!("Speaker actuator task spawned");

    // ========================================================================
    // 6. Spawn Command Distributor Task
    // ========================================================================

    log::info!("Spawning command distributor task...");

    // Clone LED channels for manual loading state override
    let red_led_tx_clone = red_led_tx.clone();
    let green_led_tx_clone = green_led_tx.clone();

    let distributor_task = {
        let shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            if let Err(e) = command_distributor::run_command_distributor(
                controller_cmd_rx,
                rgb_led_tx,
                speaker_tx,
                green_led_tx,
                red_led_tx,
                audio_cmd_tx,
                shutdown_rx,
            )
            .await
            {
                log::error!("Command distributor task failed: {}", e);
            }
        })
    };

    log::info!("Command distributor task spawned");

    // ========================================================================
    // 7. Spawn AI Controller Task (Heavy: LLM service initialization)
    // ========================================================================

    log::info!("Spawning AI controller task...");

    let controller_task = {
        let shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            if let Err(e) =
                ai_controller::run_controller(event_rx, controller_cmd_tx, shutdown_rx, config)
                    .await
            {
                log::error!("AI controller task failed: {}", e);
            }
        })
    };

    log::info!("AI controller task spawned");

    // ========================================================================
    // 8. Override Initial State with Loading Indicators
    // ========================================================================

    // Wait for controller to send its initial Ready state commands
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    log::info!("System loading - heavy components initializing (Vosk, Piper)...");

    // Override with loading state: Red LEDs breathing = system loading
    if let Err(e) = red_led_tx_clone
        .send(Command::SetRedLeds(StatusLedPattern::Breathing))
        .await
    {
        log::warn!("Failed to send loading red LED command: {}", e);
    }

    // Green LEDs off during loading
    if let Err(e) = green_led_tx_clone
        .send(Command::SetGreenLeds(StatusLedPattern::Off))
        .await
    {
        log::warn!("Failed to send loading green LED command: {}", e);
    }

    // The controller will automatically transition LEDs back to Ready state (green breathing)
    // when it completes initialization and starts processing events

    // ========================================================================
    // 9. System Ready Message
    // ========================================================================

    log::info!("");
    log::info!("==============================================================");
    log::info!("                   System is READY!                         ");
    log::info!("  Say 'hey' to wake Pi Bot and start a conversation        ");
    log::info!("  Press Ctrl+C to shutdown gracefully                       ");
    log::info!("==============================================================");
    log::info!("");

    // ========================================================================
    // 10. Wait for Shutdown Signal (Ctrl+C)
    // ========================================================================

    tokio::signal::ctrl_c().await?;

    log::info!("");
    log::info!("Shutdown signal received, stopping all tasks...");

    // Broadcast shutdown to all tasks
    let _ = shutdown_tx.send(());

    // Wait for all tasks to complete (with timeout)
    let shutdown_timeout = tokio::time::Duration::from_secs(5);
    tokio::select! {
        _ = tokio::time::sleep(shutdown_timeout) => {
            log::warn!("Shutdown timeout reached, some tasks may not have stopped cleanly");
        }
        _ = async {
            let _ = tokio::join!(
                pir_task,
                audio_task,
                rgb_led_task,
                speaker_task,
                green_led_task,
                red_led_task,
                distributor_task,
                controller_task,
            );
        } => {
            log::info!("All tasks stopped successfully");
        }
    }

    log::info!("");
    log::info!("==============================================================");
    log::info!("            Pi Bot Companion System Stopped                 ");
    log::info!("==============================================================");
    log::info!("");

    Ok(())
}
