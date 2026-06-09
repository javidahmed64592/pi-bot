//! # runner
//!
//! Main orchestration binary for the Pi Bot companion system.
//!
//! ## Startup Sequence
//!
//! The runner follows a strictly ordered startup sequence so that visual
//! loading indicators are correct and the AI controller owns all LED state:
//!
//! ```text
//! 1. Initialise channels
//! 2. Spawn AI controller + command distributor
//! 3. Spawn actuators  →  each emits ComponentReady  →  controller sets red LEDs (loading)
//! 4. Spawn sensors    →  each emits ComponentReady  →  controller sets green LEDs (ready)
//! 5. Main loop (Ctrl+C to shutdown)
//! 6. Shutdown signal  →  controller turns all components off
//! ```
//!
//! Components signal readiness via a shared `startup_tx` channel. The runner
//! forwards each signal as an `Event::ComponentReady` to the controller, which
//! owns the LED state transitions. The runner never sends commands directly to
//! actuators — all actuator control goes through the controller.

mod audio_sensor;
mod command_distributor;
mod green_led_actuator;
mod lcd_actuator;
mod pir_sensor;
mod red_led_actuator;
mod rgb_led_actuator;
mod speaker_actuator;

use anyhow::Result;
use bot_core::{load_config, Event};
use tokio::sync::{broadcast, mpsc};

/// Number of actuator components expected to report ready before sensors are spawned.
const ACTUATOR_COUNT: usize = 5; // rgb_led, green_led, red_led, speaker, lcd

/// Number of sensor components expected to report ready before the system is fully ready.
const SENSOR_COUNT: usize = 2; // pir, audio

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
    // 2. Initialise Channels
    // ========================================================================

    log::info!("Initializing communication channels...");

    // Event channel: Sensors + startup signals → Controller
    let (event_tx, event_rx) = mpsc::channel::<Event>(64);

    // Startup readiness channel: Components → Runner (runner forwards as ComponentReady events)
    // Each component sends (component_name, is_healthy) when it finishes initialising.
    let (startup_tx, mut startup_rx) = mpsc::channel::<(String, bool)>(16);

    // Controller command channel: Controller → Command Distributor
    let (controller_cmd_tx, controller_cmd_rx) = mpsc::channel::<bot_core::Command>(32);

    // Actuator command channels: Command Distributor → Actuators
    let (rgb_led_tx, rgb_led_rx) = mpsc::channel::<bot_core::Command>(32);
    let (speaker_tx, speaker_rx) = mpsc::channel::<bot_core::Command>(32);
    let (green_led_tx, green_led_rx) = mpsc::channel::<bot_core::Command>(32);
    let (red_led_tx, red_led_rx) = mpsc::channel::<bot_core::Command>(32);
    let (lcd_tx, lcd_rx) = mpsc::channel::<bot_core::Command>(32);

    // Audio sensor command channel: Command Distributor → Audio Sensor
    let (audio_cmd_tx, audio_cmd_rx) = mpsc::channel::<audio_sensor::AudioSensorCommand>(32);

    // Shutdown channel (broadcast to all tasks)
    let (shutdown_tx, _) = broadcast::channel::<()>(16);

    log::info!("Channels initialized");

    // ========================================================================
    // 3. Spawn AI Controller + Command Distributor
    //
    // These must be running before actuators are spawned so they can start
    // receiving ComponentReady events and routing commands immediately.
    // ========================================================================

    log::info!("Spawning AI controller...");

    let controller_task = {
        let config = config.clone(); // controller takes its own clone
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

    log::info!("Spawning command distributor...");

    let distributor_task = {
        let shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            if let Err(e) = command_distributor::run_command_distributor(
                controller_cmd_rx,
                rgb_led_tx,
                speaker_tx,
                green_led_tx,
                red_led_tx,
                lcd_tx,
                audio_cmd_tx,
                shutdown_rx,
            )
            .await
            {
                log::error!("Command distributor task failed: {}", e);
            }
        })
    };

    log::info!("Controller and distributor spawned");

    // ========================================================================
    // 4. Spawn Actuator Tasks
    //
    // Actuators are spawned next. Each signals readiness via startup_tx once
    // its hardware is initialised. The runner collects these signals and
    // forwards them as ComponentReady events so the controller can activate
    // the red loading LEDs and track startup progress.
    //
    // Red + RGB LED actuators default to a red-breathing pattern on startup so
    // the hardware shows a loading state even before the controller's first
    // command arrives.
    // ========================================================================

    log::info!("Spawning actuator tasks...");

    let rgb_led_task = {
        let config = config.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        let startup = startup_tx.clone();
        tokio::spawn(async move {
            if let Err(e) =
                rgb_led_actuator::run_rgb_led_actuator(&config, rgb_led_rx, startup, shutdown_rx)
                    .await
            {
                log::error!("RGB LED actuator task failed: {}", e);
            }
        })
    };

    let green_led_task = {
        let config = config.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        let startup = startup_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = green_led_actuator::run_green_led_actuator(
                &config,
                green_led_rx,
                startup,
                shutdown_rx,
            )
            .await
            {
                log::error!("Green LED actuator task failed: {}", e);
            }
        })
    };

    let red_led_task = {
        let config = config.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        let startup = startup_tx.clone();
        tokio::spawn(async move {
            if let Err(e) =
                red_led_actuator::run_red_led_actuator(&config, red_led_rx, startup, shutdown_rx)
                    .await
            {
                log::error!("Red LED actuator task failed: {}", e);
            }
        })
    };

    let speaker_task = {
        let config = config.clone();
        let event_tx_speaker = event_tx.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        let startup = startup_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = speaker_actuator::run_speaker_actuator(
                &config,
                speaker_rx,
                event_tx_speaker,
                startup,
                shutdown_rx,
            )
            .await
            {
                log::error!("Speaker actuator task failed: {}", e);
            }
        })
    };

    let lcd_task = {
        let config = config.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        let startup = startup_tx.clone();
        tokio::spawn(async move {
            if let Err(e) =
                lcd_actuator::run_lcd_actuator(&config, lcd_rx, startup, shutdown_rx).await
            {
                log::error!("LCD actuator task failed: {}", e);
            }
        })
    };

    log::info!(
        "Actuator tasks spawned, waiting for {} components to report ready...",
        ACTUATOR_COUNT
    );

    // ========================================================================
    // 5. Wait for Actuators → Forward ComponentReady Events
    //
    // Block until every actuator has sent its startup signal. Each signal is
    // forwarded to the controller as Event::ComponentReady so the controller
    // can trigger the red loading LED state.
    // ========================================================================

    let mut actuators_ready = 0;
    while actuators_ready < ACTUATOR_COUNT {
        match startup_rx.recv().await {
            Some((name, healthy)) => {
                if healthy {
                    log::info!(
                        "Actuator ready: '{}' ({}/{})",
                        name,
                        actuators_ready + 1,
                        ACTUATOR_COUNT
                    );
                } else {
                    log::warn!(
                        "Actuator '{}' initialized in degraded mode ({}/{})",
                        name,
                        actuators_ready + 1,
                        ACTUATOR_COUNT
                    );
                }
                // Forward to controller — it decides what to do with unhealthy components
                if let Err(e) = event_tx
                    .send(Event::ComponentReady { component: name })
                    .await
                {
                    log::error!("Failed to forward ComponentReady event: {}", e);
                }
                actuators_ready += 1;
            }
            None => {
                log::error!(
                    "Startup channel closed unexpectedly after {}/{} actuators",
                    actuators_ready,
                    ACTUATOR_COUNT
                );
                break;
            }
        }
    }

    log::info!("All actuators ready — spawning sensor tasks");

    // ========================================================================
    // 6. Spawn Sensor Tasks
    //
    // Sensors are spawned after all actuators are ready, matching the diagram.
    // The audio sensor is the heavy component (Vosk model loading). Its ready
    // signal is the final step before the system transitions to green LEDs.
    // ========================================================================

    let pir_task = {
        let config = config.clone();
        let event_tx_pir = event_tx.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        let startup = startup_tx.clone();
        tokio::spawn(async move {
            if let Err(e) =
                pir_sensor::run_pir_sensor(&config, event_tx_pir, startup, shutdown_rx).await
            {
                log::error!("PIR sensor task failed: {}", e);
            }
        })
    };

    let audio_task = {
        let config = config.clone();
        let event_tx_audio = event_tx.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        let startup = startup_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = audio_sensor::run_audio_sensor(
                &config,
                event_tx_audio,
                audio_cmd_rx,
                shutdown_rx,
                startup,
            )
            .await
            {
                log::error!("Audio sensor task failed: {}", e);
            }
        })
    };

    log::info!(
        "Sensor tasks spawned, waiting for {} sensors to report ready (Vosk loading...)...",
        SENSOR_COUNT
    );

    // ========================================================================
    // 7. Wait for Sensors → Forward ComponentReady Events
    //
    // Wait for all sensors to signal readiness and forward to the controller.
    // When the controller receives the final ComponentReady it transitions the
    // LEDs from red breathing (loading) to green solid (ready).
    // ========================================================================

    let mut sensors_ready = 0;
    while sensors_ready < SENSOR_COUNT {
        match startup_rx.recv().await {
            Some((name, healthy)) => {
                if healthy {
                    log::info!(
                        "Sensor ready: '{}' ({}/{})",
                        name,
                        sensors_ready + 1,
                        SENSOR_COUNT
                    );
                } else {
                    log::warn!(
                        "Sensor '{}' initialized in degraded mode ({}/{})",
                        name,
                        sensors_ready + 1,
                        SENSOR_COUNT
                    );
                }
                if let Err(e) = event_tx
                    .send(Event::ComponentReady { component: name })
                    .await
                {
                    log::error!("Failed to forward ComponentReady event: {}", e);
                }
                sensors_ready += 1;
            }
            None => {
                log::error!(
                    "Startup channel closed unexpectedly after {}/{} sensors",
                    sensors_ready,
                    SENSOR_COUNT
                );
                break;
            }
        }
    }

    // ========================================================================
    // 8. System Ready
    // ========================================================================

    log::info!("");
    log::info!("==============================================================");
    log::info!("                   System is READY!                         ");
    log::info!("  Say 'hey' to wake Pi Bot and start a conversation        ");
    log::info!("  Press Ctrl+C to shutdown gracefully                       ");
    log::info!("==============================================================");
    log::info!("");

    // ========================================================================
    // 9. Wait for Shutdown Signal (Ctrl+C)
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
                lcd_task,
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
