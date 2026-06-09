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
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

use crate::{
    llm_service::Message, memory_service::FactSource, observation_mode::ObservationContext,
    LlmService, MemoryService,
};

// ============================================================================
// Startup Tracking
// ============================================================================

/// Expected actuator component names (must report ready before sensors are spawned)
const ACTUATOR_COMPONENTS: &[&str] = &["rgb_led", "green_led", "red_led", "speaker", "lcd"];

/// Expected sensor component names (must all report ready for system to be fully ready)
const SENSOR_COMPONENTS: &[&str] = &["pir", "audio"];

/// Tracks which components have reported ready during system startup.
///
/// The startup sequence has two phases:
/// 1. Actuator phase: red LEDs breathing (loading), waiting for sensors
/// 2. Sensor phase: all components ready → green LEDs solid (ready)
#[derive(Default)]
struct StartupTracker {
    /// Names of all components that have sent ComponentReady
    ready_components: HashSet<String>,
    /// Whether all components (actuators + sensors) have reported ready
    all_components_ready: bool,
}

impl StartupTracker {
    fn new() -> Self {
        Self::default()
    }

    /// Mark a component as ready. Returns true if this was the final component.
    fn mark_ready(&mut self, component: String) -> bool {
        self.ready_components.insert(component);

        let all_ready = ACTUATOR_COMPONENTS
            .iter()
            .chain(SENSOR_COMPONENTS.iter())
            .all(|&name| self.ready_components.contains(name));

        if all_ready && !self.all_components_ready {
            self.all_components_ready = true;
        }

        self.all_components_ready
    }

    /// Return names of components still awaited
    fn pending(&self) -> Vec<&str> {
        ACTUATOR_COMPONENTS
            .iter()
            .chain(SENSOR_COMPONENTS.iter())
            .filter_map(|&name| {
                if self.ready_components.contains(name) {
                    None
                } else {
                    Some(name)
                }
            })
            .collect()
    }
}

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

    /// Is speech capture currently active (user is speaking)?
    speech_capture_active: bool,

    /// When did we last detect presence? (for PIR timeout in Silent mode)
    last_presence_time: Instant,

    /// Pending LCD command to send when speech playback starts
    pending_lcd_command: Option<Command>,

    /// Pending fact extraction: (user_msg, assistant_response) from the last exchange.
    ///
    /// Set after each conversation turn and processed in `handle_speech_complete`,
    /// so extraction runs while the bot is transitioning back to Ready — never
    /// blocking the user from getting a response.
    pending_extraction: Option<(String, String)>,

    /// Tracks component readiness during startup
    startup: StartupTracker,

    /// System configuration
    config: SystemConfig,

    /// How long the user has been continuously present (updated from DeskPresenceDuration events)
    presence_duration_minutes: u32,
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
            speech_capture_active: false,
            last_presence_time: Instant::now(),
            pending_lcd_command: None,
            pending_extraction: None,
            startup: StartupTracker::new(),
            config,
            presence_duration_minutes: 0,
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
// Response Parsing Helpers
// ============================================================================

/// Parsed LLM response containing spoken text and optional commands
#[derive(Debug)]
struct ParsedResponse {
    /// Text to be spoken via TTS
    spoken_text: String,
    /// Optional LCD display command
    lcd_command: Option<Command>,
}

/// Parse LLM response to extract spoken text and commands
///
/// Format: "Some spoken text\nCOMMAND: DisplayText|<line1>|<line2>|<duration_ms>"
/// Handles markdown formatting and extracts clean command
/// Only extracts the FIRST command found to prevent multiple LCD displays
fn parse_llm_response(response: &str) -> ParsedResponse {
    let mut spoken_text = response.to_string();
    let mut lcd_command = None;

    // Look for FIRST COMMAND: prefix (may be wrapped in markdown **)
    if let Some(cmd_start) = response.find("COMMAND:") {
        // Get text before command (strip trailing markdown/whitespace)
        spoken_text = response[..cmd_start]
            .trim()
            .trim_end_matches('*')
            .trim()
            .to_string();

        // Extract command line (everything from COMMAND: to end of line or next **)
        let after_command = &response[cmd_start + 8..]; // Skip "COMMAND:"
        let command_line = after_command
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('*') // Strip markdown bold
            .trim();

        debug!("Extracted command line: '{}'", command_line);

        // Parse DisplayText command: DisplayText|<line1>|<line2>|<duration_ms>
        if let Some(display_cmd) = command_line.strip_prefix("DisplayText|") {
            let parts: Vec<&str> = display_cmd.split('|').collect();

            if parts.len() >= 3 {
                let line1 = parts[0].to_string();
                let line2 = parts[1].to_string();
                let duration_str = parts[2].trim();

                // Parse duration (handle potential errors gracefully)
                if let Ok(duration_ms) = duration_str.parse::<u64>() {
                    lcd_command = Some(Command::DisplayText {
                        line1: line1.clone(),
                        line2: line2.clone(),
                        duration_ms: Some(duration_ms),
                    });

                    info!(
                        "Parsed LCD command: line1='{}', line2='{}', duration={}ms",
                        line1, line2, duration_ms
                    );
                } else {
                    warn!(
                        "Failed to parse duration '{}' from command line: '{}'",
                        duration_str, command_line
                    );
                }
            } else {
                warn!(
                    "Invalid DisplayText command format (expected 3 parts, got {}): '{}'",
                    parts.len(),
                    command_line
                );
            }
        }
    }

    ParsedResponse {
        spoken_text,
        lcd_command,
    }
}

// ============================================================================
// Memory Command Handling
// ============================================================================

/// Explicit memory commands parsed from user speech before the LLM sees them.
///
/// These are handled transparently: the fact database is updated first, then
/// the user's original message is passed to the LLM so it can respond naturally
/// (e.g. "I'll remember that!" or "I've forgotten that.").
enum MemoryCommandType {
    /// User asked the bot to remember a specific fact
    StoreFact(String),
    /// User asked the bot to forget something
    ForgetFact(String),
}

/// Detect whether the user's message is an explicit memory command using the LLM.
///
/// Uses a fast, low-temperature LLM call to detect natural language intent.
/// Catches variations like "remember that", "don't forget", "keep in mind",
/// "you should know", etc. Returns None silently on any error.
async fn detect_memory_command(text: &str, llm: &LlmService) -> Option<MemoryCommandType> {
    // Skip very short messages
    if text.split_whitespace().count() < 3 {
        return None;
    }

    let prompt = format!(
        "Analyze this user message and detect if they are explicitly asking you to remember or forget something.\n\n\
         User message: \"{}\"\n\n\
         Respond with ONLY a JSON object (no markdown):  \n\
         {{\"intent\": \"remember\" | \"forget\" | \"none\", \"content\": \"extracted fact or query\"}}\n\n\
         Examples:\n\
         - \"Can you remember that I like coffee?\" → {{\"intent\": \"remember\", \"content\": \"I like coffee\"}}\n\
         - \"Don't forget I work from home\" → {{\"intent\": \"remember\", \"content\": \"I work from home\"}}\n\
         - \"Forget that I like tea\" → {{\"intent\": \"forget\", \"content\": \"I like tea\"}}\n\
         - \"What's the weather like?\" → {{\"intent\": \"none\", \"content\": \"\"}}\n\n\
         JSON response:",
        text
    );

    let messages = vec![
        Message::system(
            "You detect memory intent from user messages. \
             Respond ONLY with JSON, no markdown formatting.",
        ),
        Message::user(&prompt),
    ];

    let response = llm.generate_with_options(&messages, 0.0, 256).await.ok()?;

    // Parse JSON response
    #[derive(serde::Deserialize)]
    struct IntentResponse {
        intent: String,
        content: String,
    }

    let json_str = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let intent: IntentResponse = serde_json::from_str(json_str).ok()?;

    match intent.intent.to_lowercase().as_str() {
        "remember" if !intent.content.is_empty() => {
            debug!("Detected memory store intent: '{}'", intent.content);
            Some(MemoryCommandType::StoreFact(intent.content))
        }
        "forget" if !intent.content.is_empty() => {
            debug!("Detected memory forget intent: '{}'", intent.content);
            Some(MemoryCommandType::ForgetFact(intent.content))
        }
        _ => None,
    }
}

/// Apply any explicit memory command BEFORE sending the message to the LLM.
///
/// This updates the fact database silently so the LLM can then respond
/// naturally to the user's request ("I'll remember that!").
async fn preprocess_memory_commands(text: &str, state: &mut ControllerState) {
    let Some(cmd) = detect_memory_command(text, &state.llm).await else {
        return;
    };

    match cmd {
        MemoryCommandType::StoreFact(fact_text) => {
            info!("Storing user-told fact: '{}'", fact_text);
            match state
                .memory
                .add_fact(fact_text.clone(), FactSource::UserTold, None)
                .await
            {
                Ok(_) => info!("Fact stored successfully"),
                Err(e) => warn!("Failed to store fact (may be duplicate): {}", e),
            }
        }

        MemoryCommandType::ForgetFact(query) => {
            info!("User wants to forget: '{}'", query);
            if state.memory.has_semantic_memory() {
                match state.memory.search_facts(&query, 1, 0.65).await {
                    Ok(results) if !results.is_empty() => {
                        let (fact, score) = &results[0];
                        info!(
                            "Found fact to forget (similarity={:.2}): '{}'",
                            score, fact.text
                        );
                        let fact_id = fact.id.clone();
                        if let Err(e) = state.memory.remove_fact(&fact_id) {
                            warn!("Failed to remove fact: {}", e);
                        } else {
                            info!("Fact removed from long-term memory");
                        }
                    }
                    _ => {
                        info!("No matching fact found to forget");
                    }
                }
            } else {
                warn!("Semantic memory not available — cannot search for fact to forget");
            }
        }
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

    // Send loading state: red LEDs breathing, green LEDs off, RGB off.
    // Actuators default to this on startup, but we reinforce it here in case
    // the controller finishes loading before actuators emit ComponentReady.
    if let Err(e) = send_loading_commands(&cmd_tx).await {
        error!("Failed to send loading commands: {}", e);
    }

    // Create timeout ticker for periodic checks
    let mut timeout_check_interval = tokio::time::interval(Duration::from_secs(1));

    info!("AI controller ready, waiting for components to report in");

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
        Event::SpeechCaptureStarted => handle_speech_capture_started(state, cmd_tx).await?,
        Event::SpeechCaptured(text) => handle_speech_captured(text, state, cmd_tx).await?,
        Event::SpeechPlaybackStarted => handle_speech_playback_started(state, cmd_tx).await?,
        Event::SpeechComplete => handle_speech_complete(state, cmd_tx).await?,
        Event::PresenceDetected => handle_presence_detected(state, cmd_tx).await?,
        Event::NoPresenceSince(duration) => handle_no_presence(duration, state, cmd_tx).await?,
        Event::DeskPresenceDuration(minutes) => {
            handle_desk_presence_duration(minutes, state, cmd_tx).await?
        }
        Event::AmbientNoiseLevel(level) => handle_ambient_noise(level, state, cmd_tx).await?,
        Event::UserRequestedDND => handle_user_requested_dnd(state, cmd_tx).await?,
        Event::UserRequestedWakeUp => handle_user_requested_wakeup(state, cmd_tx).await?,
        Event::ComponentReady { component } => {
            handle_component_ready(component, state, cmd_tx).await?
        }
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

/// Handle speech capture started
async fn handle_speech_capture_started(
    state: &mut ControllerState,
    _cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    info!("Speech capture started - user is speaking");

    // Mark that speech capture is active
    state.speech_capture_active = true;

    // Update interaction timer to prevent timeout during speech
    state.mark_interaction();

    Ok(())
}

/// Handle speech playback started
async fn handle_speech_playback_started(
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    debug!("Speech playback started");

    // Send pending LCD command now that speech is starting
    if let Some(lcd_cmd) = state.pending_lcd_command.take() {
        info!("Sending LCD command synchronized with speech start");
        cmd_tx
            .send(lcd_cmd)
            .await
            .context("Failed to send LCD command")?;
    }

    Ok(())
}

/// Handle speech playback completion
async fn handle_speech_complete(
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    info!("Speech playback complete");

    // Only transition if we're actually in Speaking state
    if !matches!(
        state.bot_state.conversation_state,
        ConversationState::Active(ActiveSubState::Speaking)
    ) {
        debug!(
            "Received SpeechComplete but not in Speaking state (current: {:?})",
            state.bot_state.conversation_state
        );
        return Ok(());
    }

    // Run pending fact extraction before returning to Ready.
    // We briefly show the Learning LED to give visual feedback during extraction.
    // This happens after the bot finishes speaking so the user never waits for it.
    if let Some((user_msg, assistant_msg)) = state.pending_extraction.take() {
        debug!("Processing pending fact extraction");

        // Visual cue: Learning state during extraction
        state.bot_state.conversation_state = ConversationState::Active(ActiveSubState::Learning);
        send_state_commands(cmd_tx, state).await?;

        let extracted_facts = state.llm.extract_facts(&user_msg, &assistant_msg).await;

        if extracted_facts.is_empty() {
            debug!("No facts extracted from exchange");
        } else {
            info!(
                "Extracted {} fact(s) from exchange — storing in long-term memory",
                extracted_facts.len()
            );
            for fact_text in extracted_facts {
                match state
                    .memory
                    .add_fact(fact_text.clone(), FactSource::Conversation, None)
                    .await
                {
                    Ok(fact) => debug!("Stored fact [{}]: '{}'", &fact.id[..8], fact.text),
                    Err(e) => debug!("Fact skipped (duplicate or error): {}", e),
                }
            }
        }
    }

    // Now return to ready state
    return_to_ready_state(state, cmd_tx).await?;
    Ok(())
}

/// Handle a component reporting that it has finished initialisation
///
/// The controller tracks two startup phases:
/// 1. All actuators ready → red LEDs breathing (loading sensors)
/// 2. All components ready → green LEDs solid (system ready)
async fn handle_component_ready(
    component: String,
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    info!("Component ready: '{}'", component);

    let all_ready = state.startup.mark_ready(component.clone());

    let ready_count =
        ACTUATOR_COMPONENTS.len() + SENSOR_COMPONENTS.len() - state.startup.pending().len();
    let total = ACTUATOR_COMPONENTS.len() + SENSOR_COMPONENTS.len();

    info!(
        "Component ready: '{}' ({}/{})",
        component, ready_count, total
    );

    if all_ready {
        info!("All components ready — transitioning to Ready state");

        state.bot_state.conversation_state = ConversationState::Ready;

        // Green LEDs solid = system ready; red LEDs off; RGB LED to Ready pattern
        send_state_commands(cmd_tx, state).await?;

        info!("System is READY. Say 'hey' to wake Pi Bot.");
    } else {
        let pending = state.startup.pending();
        info!("Still waiting for: {:?}", pending);
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
    state.speech_capture_active = false; // Speech capture complete

    // Pre-process explicit memory commands (remember / forget) so the fact
    // database is updated before the LLM sees the message. The LLM then
    // responds naturally to the original phrasing.
    preprocess_memory_commands(&text, state).await;

    // Transition to thinking state
    state.bot_state.conversation_state = ConversationState::Active(ActiveSubState::Thinking);
    send_state_commands(cmd_tx, state).await?;

    // Stop listening while thinking to save resources
    cmd_tx
        .send(Command::StopListening)
        .await
        .context("Failed to send StopListening command")?;

    // Build conversation context.
    // When semantic memory is enabled, augment with relevant long-term facts
    // so the LLM can reference what it knows about the user.
    let history =
        if state.memory.has_semantic_memory() && state.config.memory.fact_extraction_enabled {
            state.memory.get_context_with_facts(&text).await
        } else {
            state.memory.get_context()
        };

    // Query LLM
    info!("Querying LLM...");
    let response = match state.llm.generate(&text, &history).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("LLM generation failed: {}", e);
            let error_msg = "Sorry, I'm having trouble thinking right now. Can you try again?";

            // Transition to speaking state
            state.bot_state.conversation_state =
                ConversationState::Active(ActiveSubState::Speaking);
            send_state_commands(cmd_tx, state).await?;

            // Stop listening to prevent bot from listening to itself
            cmd_tx
                .send(Command::StopListening)
                .await
                .context("Failed to send StopListening command")?;

            // Send error response via TTS
            cmd_tx
                .send(Command::Speak {
                    text: error_msg.to_string(),
                })
                .await
                .context("Failed to send error speech command")?;

            // Stay in Speaking state - we'll transition to Ready when we receive SpeechComplete
            return Ok(());
        }
    };

    info!("LLM response: '{}'", response);

    // Parse response to extract spoken text and commands
    let parsed = parse_llm_response(&response);

    // Transition to learning state (brief, store in memory)
    state.bot_state.conversation_state = ConversationState::Active(ActiveSubState::Learning);
    send_state_commands(cmd_tx, state).await?;

    // Store exchange in memory (store original response with commands for context)
    state.memory.add_exchange(text.clone(), response.clone());

    // Queue fact extraction to run AFTER speech completes so it never adds
    // latency to the user experience. Only queued when extraction is enabled.
    if state.config.memory.fact_extraction_enabled && state.memory.has_semantic_memory() {
        state.pending_extraction = Some((text, response.clone()));
        debug!("Queued fact extraction for post-speech processing");
    }

    // Transition to speaking state
    state.bot_state.conversation_state = ConversationState::Active(ActiveSubState::Speaking);
    send_state_commands(cmd_tx, state).await?;

    // Stop listening to prevent bot from listening to itself
    cmd_tx
        .send(Command::StopListening)
        .await
        .context("Failed to send StopListening command")?;

    // Store LCD command to send when speech playback starts (for better timing)
    if let Some(lcd_cmd) = parsed.lcd_command {
        info!("Storing LCD command to send when speech starts");
        state.pending_lcd_command = Some(lcd_cmd);
    }

    // Send TTS command (speak the parsed text without the command part)
    cmd_tx
        .send(Command::Speak {
            text: parsed.spoken_text,
        })
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
        // Reset presence duration so the bot gives the user a moment to settle
        // before the PIR sensor triggers an observation
        state.presence_duration_minutes = 0;
        send_state_commands(cmd_tx, state).await?;
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
    // Reset desk presence counter — will restart from zero when user returns
    state.presence_duration_minutes = 0;

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

/// Handle periodic desk presence duration update
///
/// This event is emitted by the PIR sensor at random intervals while the user
/// is continuously present. The controller uses this as a trigger to decide
/// whether to initiate a proactive conversation (Observing state).
async fn handle_desk_presence_duration(
    minutes: u32,
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    debug!("Desk presence duration update: {} minutes", minutes);
    state.presence_duration_minutes = minutes;

    // Trigger observation logic only once the system is fully loaded and
    // we are in the Ready state with confirmed presence.
    // Guard against observations during startup (before all components are ready).
    if state.startup.all_components_ready
        && matches!(state.bot_state.conversation_state, ConversationState::Ready)
        && state.bot_state.presence_detected
    {
        trigger_observation(state, cmd_tx).await?;
    } else if !state.startup.all_components_ready {
        debug!("Skipping observation — system not fully initialised yet");
    }

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
// Observation Logic
// ============================================================================

/// Execute a passive observation cycle.
///
/// Triggered when the PIR sensor emits a DeskPresenceDuration event and the bot
/// is in the Ready state with presence detected.
///
/// 1. Transitions to `Observing` state (blue breathing LED).
/// 2. Builds an [`ObservationContext`] from current system state.
/// 3. Decides probabilistically whether to speak.
/// 4. If yes: generates a conversation opener via LLM and speaks it.
/// 5. If no: returns to `Ready` and waits for the next event.
async fn trigger_observation(
    state: &mut ControllerState,
    cmd_tx: &mpsc::Sender<Command>,
) -> Result<()> {
    info!("[Observation] Presence check received — entering Observing state");

    // Transition to Observing state
    state.bot_state.conversation_state = ConversationState::Observing;
    send_state_commands(cmd_tx, state).await?;

    // Collect context — prefer semantically relevant facts when available
    let recent_facts: Vec<String> = if state.memory.has_semantic_memory() {
        // Use a general "user context" query to surface diverse relevant facts
        let observation_query = "user preferences habits work activities interests";
        match state.memory.search_facts(observation_query, 5, 0.35).await {
            Ok(results) if !results.is_empty() => {
                results.into_iter().map(|(f, _)| f.text).collect()
            }
            _ => {
                // Fall back to most recent facts
                state
                    .memory
                    .get_all_facts()
                    .iter()
                    .rev()
                    .take(5)
                    .map(|f| f.text.clone())
                    .collect()
            }
        }
    } else {
        state
            .memory
            .get_all_facts()
            .iter()
            .rev()
            .take(5)
            .map(|f| f.text.clone())
            .collect()
    };

    let ctx = ObservationContext::new(
        state.presence_duration_minutes,
        state.last_interaction.elapsed(),
        recent_facts,
    );

    // Decide whether to initiate
    if !ctx.should_initiate(&state.config.behavior.observation_probability) {
        info!("[Observation] Decision: stay quiet");
        return_to_ready_state(state, cmd_tx).await?;
        return Ok(());
    }

    info!("[Observation] Decision: initiate conversation");

    // Build opener prompt and pass through the normal generation pipeline
    let prompt = ctx.build_opener_prompt();
    let history = state.memory.get_context();

    // Transition to Thinking while generating
    state.bot_state.conversation_state = ConversationState::Active(ActiveSubState::Thinking);
    send_state_commands(cmd_tx, state).await?;

    let response = match state.llm.generate(&prompt, &history).await {
        Ok(r) => r,
        Err(e) => {
            error!("[Observation] LLM generation failed: {}", e);
            return_to_ready_state(state, cmd_tx).await?;
            return Ok(());
        }
    };

    info!("[Observation] Opener: '{}'", response);

    // Parse for optional LCD command embedded in response
    let parsed = parse_llm_response(&response);

    // Brief Learning state to store the exchange
    state.bot_state.conversation_state = ConversationState::Active(ActiveSubState::Learning);
    send_state_commands(cmd_tx, state).await?;

    // Record as a bot-initiated exchange so memory is consistent
    state
        .memory
        .add_exchange("[bot-initiated]".to_string(), response.clone());

    // Transition to Speaking
    state.bot_state.conversation_state = ConversationState::Active(ActiveSubState::Speaking);
    send_state_commands(cmd_tx, state).await?;

    // Stop microphone so the bot doesn't listen to its own voice
    cmd_tx
        .send(Command::StopListening)
        .await
        .context("Failed to send StopListening command")?;

    // Queue LCD command (sent on SpeechPlaybackStarted for timing)
    if let Some(lcd_cmd) = parsed.lcd_command {
        state.pending_lcd_command = Some(lcd_cmd);
    }

    // Speak the opener — SpeechComplete will return us to Ready
    cmd_tx
        .send(Command::Speak {
            text: parsed.spoken_text,
        })
        .await
        .context("Failed to send Speak command")?;

    // Mark interaction so the observation probability resets
    state.mark_interaction();

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
    // Only check timeout if we're in Listening state (waiting for user input)
    // Don't timeout while Thinking, Speaking, or Learning
    if !matches!(
        state.bot_state.conversation_state,
        ConversationState::Active(ActiveSubState::Listening)
    ) {
        return Ok(());
    }

    // Don't timeout if speech capture is currently active (user is speaking)
    if state.speech_capture_active {
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
    state.speech_capture_active = false;

    // Resume listening for wake word
    cmd_tx
        .send(Command::StartListening)
        .await
        .context("Failed to send StartListening command")?;

    // Send state commands
    send_state_commands(cmd_tx, state).await?;

    Ok(())
}

// ============================================================================
// Command Generation
// ============================================================================

/// Send loading state commands when controller first starts.
///
/// Red LEDs breathing + green off + RGB off signals that the system is
/// initializing. The actuators also default to this state on startup so
/// there is no visual gap before the controller's first command arrives.
async fn send_loading_commands(cmd_tx: &mpsc::Sender<Command>) -> Result<()> {
    info!("Sending loading state commands (red breathing, green off, RGB off)");

    cmd_tx
        .send(Command::SetRedLeds(StatusLedPattern::Breathing))
        .await
        .context("Failed to send red LED loading command")?;

    cmd_tx
        .send(Command::SetGreenLeds(StatusLedPattern::Off))
        .await
        .context("Failed to send green LED off command")?;

    cmd_tx
        .send(Command::LedOff)
        .await
        .context("Failed to send RGB LED off command")?;

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
