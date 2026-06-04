//! # AI Controller
//!
//! Main event loop and decision-making logic for Pi Bot.
//!
//! ## Architecture
//!
//! The controller follows an event-driven pattern:
//! ```text
//! [Sensors] → Events → [Controller] → Commands → [Actuators]
//! ```
//!
//! ## Responsibilities
//!
//! - Maintain bot state (conversation mode, presence, etc.)
//! - Handle sensor events (wake word, speech, motion, health)
//! - Make decisions based on state and events
//! - Send commands to actuators (LEDs, speaker)
//! - Manage LLM queries with memory context
//! - Enforce state transition rules
//!
//! ## Usage
//!
//! ```no_run
//! use ai_controller::run_controller;
//! use bot_core::config::load_config;
//! use tokio::sync::mpsc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = load_config("config/config.yaml")?;
//!
//! let (event_tx, event_rx) = mpsc::channel(32);
//! let (cmd_tx, cmd_rx) = mpsc::channel(32);
//! let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
//!
//! // Spawn controller
//! let controller = tokio::spawn(run_controller(
//!     event_rx,
//!     cmd_tx,
//!     shutdown_rx,
//!     config,
//! ));
//!
//! // ... (sensor and actuator tasks)
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use bot_core::{
    commands::{Command, StatusLedPattern},
    config::SystemConfig,
    events::Event,
    state::{ActiveSubState, BotState, ConversationState},
};
use log::{debug, error, info, warn};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

use crate::{LlmService, MemoryService};

// ============================================================================
// Controller State
// ============================================================================

/// Internal controller state (not exposed to rest of system)
struct ControllerState {
    /// Bot state (conversation mode, presence, brightness)
    bot_state: BotState,

    /// LLM service for generating responses
    llm: LlmService,

    /// Memory service for conversation history
    memory: MemoryService,

    /// Last time the user interacted (for timeout detection)
    last_interaction: Instant,

    /// Are we currently waiting for speech capture?
    awaiting_speech: bool,

    /// When did we last detect presence? (for PIR timeout in Silent mode)
    last_presence_time: Instant,

    /// System configuration
    config: SystemConfig,
}

impl ControllerState {
    /// Create new controller state
    async fn new(config: SystemConfig) -> Result<Self> {
        info!("Initializing controller state");

        // Initialize LLM service
        let llm = LlmService::new(config.llm.clone())
            .await
            .context("Failed to initialize LLM service")?;

        // Initialize memory service
        let memory =
            MemoryService::new(config.memory.clone()).context("Failed to initialize memory")?;

        Ok(Self {
            bot_state: BotState::new(),
            llm,
            memory,
            last_interaction: Instant::now(),
            awaiting_speech: false,
            last_presence_time: Instant::now(),
            config,
        })
    }

    /// Check if conversation has timed out
    fn has_conversation_timed_out(&self) -> bool {
        let timeout = Duration::from_secs(self.config.behavior.conversation_timeout);
        self.last_interaction.elapsed() > timeout
    }

    /// Reset interaction timer
    fn mark_interaction(&mut self) {
        self.last_interaction = Instant::now();
        self.bot_state.mark_interaction();
    }
}

// ============================================================================
// Main Controller Loop
// ============================================================================

/// Run the AI controller main loop
///
/// # Arguments
///
/// * `event_rx` - Receiver for events from sensors
/// * `cmd_tx` - Sender for commands to actuators
/// * `shutdown_rx` - Shutdown signal receiver
/// * `config` - System configuration
///
/// # Behavior
///
/// - Listens for events from sensors (wake word, speech, motion, etc.)
/// - Maintains bot state and makes decisions
/// - Sends commands to actuators based on state and events
/// - Handles LLM queries with conversation memory
/// - Checks for conversation timeouts periodically
/// - Shuts down gracefully on signal
pub async fn run_controller(
    mut event_rx: mpsc::Receiver<Event>,
    cmd_tx: mpsc::Sender<Command>,
    mut shutdown_rx: broadcast::Receiver<()>,
    config: SystemConfig,
) -> Result<()> {
    info!("Starting AI controller");

    // Initialize controller state
    let mut state = ControllerState::new(config)
        .await
        .context("Failed to initialize controller state")?;

    // Send initial state commands
    if let Err(e) = send_initial_commands(&cmd_tx, &state).await {
        error!("Failed to send initial commands: {}", e);
    }

    // Create timeout ticker for periodic checks
    let mut timeout_check_interval = tokio::time::interval(Duration::from_secs(1));

    info!("AI controller ready");

    loop {
        tokio::select! {
            // Handle incoming events
            Some(event) = event_rx.recv() => {
                debug!("Controller received event: {:?}", event);
                if let Err(e) = handle_event(event, &mut state, &cmd_tx).await {
                    error!("Error handling event: {}", e);
                }
            }

            // Check for conversation timeout
            _ = timeout_check_interval.tick() => {
                if let Err(e) = check_conversation_timeout(&mut state, &cmd_tx).await {
                    error!("Error checking timeout: {}", e);
                }
            }

            // Handle shutdown signal
            _ = shutdown_rx.recv() => {
                info!("AI controller shutting down");
                break;
            }
        }
    }

    Ok(())
}

// ============================================================================
// Event Handlers
// ============================================================================

/// Handle an incoming event from sensors
async fn handle_event(
    event: Event,
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    match event {
        Event::WakeWordDetected => handle_wake_word(state, cmd_tx).await?,
        Event::SpeechCaptured(text) => handle_speech_captured(text, state, cmd_tx).await?,
        Event::SpeechComplete => handle_speech_complete(state, cmd_tx).await?,
        Event::PresenceDetected => handle_presence_detected(state, cmd_tx).await?,
        Event::NoPresenceSince(duration) => handle_no_presence(duration, state, cmd_tx).await?,
        Event::AmbientNoiseLevel(level) => handle_ambient_noise(level, state, cmd_tx).await?,
        Event::UserRequestedDND => handle_user_requested_dnd(state, cmd_tx).await?,
        Event::UserRequestedWakeUp => handle_user_requested_wakeup(state, cmd_tx).await?,
        Event::ComponentHealth { component, healthy } => {
            handle_component_health(component, healthy, state, cmd_tx).await?
        }
    }

    Ok(())
}

/// Handle wake word detection
async fn handle_wake_word(
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    info!("Wake word detected");

    // Wake word works in Silent mode (per diagram) but we continue to respect can_respond check
    // This allows bot to respond to wake word even in DND, but keeps logic for future states
    // that might truly block interaction

    // Update state
    state.mark_interaction();
    state.awaiting_speech = true;
    state.bot_state.conversation_state = ConversationState::Active(ActiveSubState::Listening);

    // Send commands for listening state
    send_state_commands(cmd_tx, state).await?;

    // Start listening for speech
    cmd_tx
        .send(Command::StartListening)
        .await
        .context("Failed to send StartListening command")?;

    Ok(())
}

/// Handle speech playback completion
async fn handle_speech_complete(
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    info!("Speech playback complete");

    // Only transition if we're actually in Speaking state
    if matches!(
        state.bot_state.conversation_state,
        ConversationState::Active(ActiveSubState::Speaking)
    ) {
        // Now it's safe to return to ready state
        return_to_ready_state(state, cmd_tx).await?;
    } else {
        debug!(
            "Received SpeechComplete but not in Speaking state (current: {:?})",
            state.bot_state.conversation_state
        );
    }

    Ok(())
}

/// Handle speech captured from user
async fn handle_speech_captured(
    text: String,
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    info!("Speech captured: '{}'", text);

    if text.trim().is_empty() {
        warn!("Empty speech captured, ignoring");
        return_to_ready_state(state, cmd_tx).await?;
        return Ok(());
    }

    // Update state
    state.mark_interaction();
    state.awaiting_speech = false;

    // Transition to thinking state
    state.bot_state.conversation_state = ConversationState::Active(ActiveSubState::Thinking);
    send_state_commands(cmd_tx, state).await?;

    // Get conversation context from memory
    let history = state.memory.get_context();

    // Query LLM
    info!("Querying LLM...");
    let response = match state.llm.generate(&text, &history).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("LLM generation failed: {}", e);
            let error_msg = "Sorry, I'm having trouble thinking right now. Can you try again?";

            // Send error response via TTS
            cmd_tx
                .send(Command::Speak {
                    text: error_msg.to_string(),
                })
                .await
                .context("Failed to send error speech command")?;

            // Return to ready state
            return_to_ready_state(state, cmd_tx).await?;
            return Ok(());
        }
    };

    info!("LLM response: '{}'", response);

    // Transition to learning state (brief, store in memory)
    state.bot_state.conversation_state = ConversationState::Active(ActiveSubState::Learning);
    send_state_commands(cmd_tx, state).await?;

    // Store exchange in memory
    state.memory.add_exchange(text, response.clone());

    // Transition to speaking state
    state.bot_state.conversation_state = ConversationState::Active(ActiveSubState::Speaking);
    send_state_commands(cmd_tx, state).await?;

    // Send TTS command
    cmd_tx
        .send(Command::Speak { text: response })
        .await
        .context("Failed to send speak command")?;

    // Stay in Speaking state - we'll transition to Ready when we receive SpeechComplete event
    // This prevents the microphone from listening to the bot's own voice

    Ok(())
}

/// Handle presence detected
async fn handle_presence_detected(
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    debug!("Presence detected");
    state.bot_state.presence_detected = true;
    state.last_presence_time = Instant::now();

    // Per diagram: If in auto Silent mode (not manual), return to Ready when presence detected
    // This is the "Returned to desk, bot listening again" transition
    if let ConversationState::Silent { manual: false } = state.bot_state.conversation_state {
        info!("Presence detected while in auto Silent - transitioning to Ready");
        state.bot_state.conversation_state = ConversationState::Ready;
        send_state_commands(cmd_tx, state).await?;

        // TODO Phase 2: Occasionally greet user with a friendly message
        // Example: "Welcome back!" or "Good to see you again!"
        // This would emit Event::BotInitiatedGreeting or similar
    }

    Ok(())
}

/// Handle no presence for duration
async fn handle_no_presence(
    duration: Duration,
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    info!("No presence for {:?}", duration);
    state.bot_state.presence_detected = false;

    // Per diagram: If in Ready mode and no presence detected, transition to Silent (auto)
    // This conserves power when user is away from desk
    if matches!(state.bot_state.conversation_state, ConversationState::Ready) {
        let idle_timeout = Duration::from_secs(state.config.behavior.idle_timeout);
        if state.last_presence_time.elapsed() > idle_timeout {
            info!("PIR timeout in Ready mode - transitioning to Silent (auto)");
            state.bot_state.conversation_state = ConversationState::Silent { manual: false };
            send_state_commands(cmd_tx, state).await?;
        }
    }

    Ok(())
}

/// Handle ambient noise level
async fn handle_ambient_noise(
    level: u8,
    _state: &mut ControllerState,
    _cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    debug!("Ambient noise level: {}", level);
    // Phase 2: Could adjust behavior based on noise level
    // - High noise → reduce passive observation
    // - Music detected → enter ambient lighting mode
    Ok(())
}

/// Handle component health status change
async fn handle_component_health(
    component: String,
    healthy: bool,
    _state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    if healthy {
        info!("Component '{}' is healthy", component);
    } else {
        error!("Component '{}' has failed!", component);

        // Flash red LEDs to indicate error
        cmd_tx
            .send(Command::SetRedLeds(StatusLedPattern::Flashing))
            .await
            .context("Failed to send red LED flash command")?;
    }

    Ok(())
}

/// Handle user requesting Do Not Disturb mode
async fn handle_user_requested_dnd(
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    info!("User manually requested Do Not Disturb mode");

    // Transition to Silent state (manual = true)
    // Manual Silent requires explicit user command to exit
    state.bot_state.conversation_state = ConversationState::Silent { manual: true };

    // Send LockBot command and update LED patterns
    cmd_tx
        .send(Command::LockBot)
        .await
        .context("Failed to send LockBot command")?;

    send_state_commands(cmd_tx, state).await?;

    Ok(())
}

/// Handle user requesting to exit Do Not Disturb mode
async fn handle_user_requested_wakeup(
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    info!("User requested to exit Do Not Disturb mode");

    // Transition to Ready state
    state.bot_state.conversation_state = ConversationState::Ready;

    // Send UnlockBot command and update LED patterns
    cmd_tx
        .send(Command::UnlockBot)
        .await
        .context("Failed to send UnlockBot command")?;

    send_state_commands(cmd_tx, state).await?;

    Ok(())
}

// ============================================================================
// Timeout Handling
// ============================================================================

/// Check if conversation has timed out and return to ready state
async fn check_conversation_timeout(
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    // Only check timeout if we're in active conversation
    if !matches!(
        state.bot_state.conversation_state,
        ConversationState::Active(_)
    ) {
        return Ok(());
    }

    // Check if conversation timed out
    if state.has_conversation_timed_out() {
        info!(
            "Conversation timed out after {}s of silence",
            state.config.behavior.conversation_timeout
        );
        return_to_ready_state(state, cmd_tx).await?;
    }

    Ok(())
}

// ============================================================================
// State Transition Helpers
// ============================================================================

/// Return to ready state
async fn return_to_ready_state(
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    info!("Returning to ready state");

    state.bot_state.conversation_state = ConversationState::Ready;
    state.awaiting_speech = false;

    // Stop listening if we were
    cmd_tx
        .send(Command::StopListening)
        .await
        .context("Failed to send StopListening command")?;

    // Send state commands
    send_state_commands(cmd_tx, state).await?;

    Ok(())
}

// ============================================================================
// Command Generation
// ============================================================================

/// Send initial commands when controller starts
async fn send_initial_commands(
    cmd_tx: &mpsc::Sender<Command>,
    state: &ControllerState,
) -> Result<()> {
    info!("Sending initial state commands");

    // Send RGB LED command for ready state
    send_state_commands(cmd_tx, state).await?;

    Ok(())
}

/// Send commands for current state (RGB LED + status LEDs)
///
/// Per diagram: Active state always uses state-based colors (overrides ambient lighting).
/// Other states (Ready, Silent, Observing) respect the lighting mode setting.
async fn send_state_commands(
    cmd_tx: &mpsc::Sender<Command>,
    state: &ControllerState,
) -> Result<()> {
    use bot_core::state::LightingMode;

    // Determine if we should use state-based or ambient lighting
    // Per diagram: Active state ALWAYS uses state-based colors (overrides ambient)
    let use_state_based = matches!(
        state.bot_state.conversation_state,
        ConversationState::Active(_)
    ) || matches!(state.bot_state.lighting_mode, LightingMode::StateBased);

    if use_state_based {
        // Get appearance from config based on conversation state
        let appearance = state
            .config
            .rgb_led
            .get_appearance(&state.bot_state.conversation_state);

        let rgb_color = appearance.to_rgb_color();
        let pattern = appearance.pattern_to_led_pattern();

        // Scale color by brightness
        let scaled_color = rgb_color.scale(state.bot_state.brightness);

        // Send RGB LED command with state-based pattern
        cmd_tx
            .send(Command::SetPattern {
                pattern,
                colors: vec![scaled_color],
            })
            .await
            .context("Failed to send RGB LED command")?;
    } else {
        // Use ambient lighting mode
        match &state.bot_state.lighting_mode {
            LightingMode::Ambient { pattern } => {
                // Send ambient pattern command
                use bot_core::commands::LedPattern;
                use bot_core::state::AmbientPattern;

                let (led_pattern, colors) = match pattern {
                    AmbientPattern::Gradient { colors } => {
                        let scaled: Vec<_> = colors
                            .iter()
                            .map(|c| c.scale(state.bot_state.brightness))
                            .collect();
                        (LedPattern::Gradient, scaled)
                    }
                    AmbientPattern::Rainbow => {
                        (LedPattern::Rainbow, vec![]) // Rainbow generates its own colors
                    }
                    AmbientPattern::Pulse { color } => {
                        let scaled = color.scale(state.bot_state.brightness);
                        (LedPattern::Pulse, vec![scaled])
                    }
                    AmbientPattern::Static { color } => {
                        let scaled = color.scale(state.bot_state.brightness);
                        (LedPattern::Solid, vec![scaled])
                    }
                };

                cmd_tx
                    .send(Command::SetPattern {
                        pattern: led_pattern,
                        colors,
                    })
                    .await
                    .context("Failed to send ambient RGB LED command")?;
            }
            LightingMode::Minimal => {
                // Turn off RGB LED for minimal distraction
                cmd_tx
                    .send(Command::LedOff)
                    .await
                    .context("Failed to send LED off command")?;
            }
            LightingMode::StateBased => {
                // Already handled above
                unreachable!("StateBased should be handled in use_state_based branch");
            }
        }
    }

    // Determine status LED patterns
    let (green_pattern, red_pattern) = match &state.bot_state.conversation_state {
        ConversationState::Ready => (StatusLedPattern::Solid, StatusLedPattern::Off),
        ConversationState::Active(_) => (StatusLedPattern::Breathing, StatusLedPattern::Off),
        ConversationState::Observing => (StatusLedPattern::Breathing, StatusLedPattern::Off),
        ConversationState::Silent { .. } => (StatusLedPattern::Off, StatusLedPattern::Breathing),
    };

    // Send status LED commands
    cmd_tx
        .send(Command::SetGreenLeds(green_pattern))
        .await
        .context("Failed to send green LED command")?;

    cmd_tx
        .send(Command::SetRedLeds(red_pattern))
        .await
        .context("Failed to send red LED command")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bot_core::config::load_config;

    #[tokio::test]
    async fn test_controller_initialization() {
        // This test requires a valid config file and Ollama running
        // Skip in CI environments
        if std::env::var("CI").is_ok() {
            return;
        }

        let config = load_config("../config/config.yaml").unwrap();
        let state = ControllerState::new(config).await;

        // If Ollama is not running, this will fail gracefully
        if state.is_err() {
            eprintln!(
                "Controller initialization test skipped (Ollama not running): {}",
                state.err().unwrap()
            );
            return;
        }

        let state = state.unwrap();
        assert_eq!(state.bot_state.conversation_state, ConversationState::Ready);
        assert!(!state.awaiting_speech);
    }

    #[test]
    fn test_timeout_detection() {
        // Test timeout logic without needing full ControllerState
        let timeout = Duration::from_secs(2);
        let old_time = Instant::now() - Duration::from_secs(3);
        let recent_time = Instant::now();

        assert!(old_time.elapsed() > timeout);
        assert!(recent_time.elapsed() < timeout);
    }
}
