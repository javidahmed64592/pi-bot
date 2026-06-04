//! # bot_core
//!
//! Core types and configuration for the Pi Bot companion system.
//!
//! This crate defines:
//! - **Events**: Sensor inputs (motion, wake word, speech, environment) - Sensor → Controller
//! - **Commands**: Actuator outputs (LED colors, speech, display text) - Controller → Actuators
//! - **States**: BotState with ConversationState and LightingMode
//! - **Config**: SystemConfig loaded from YAML
//!
//! ## Example Usage
//!
//! ```no_run
//! use bot_core::{
//!     events::Event,
//!     commands::Command,
//!     state::{BotState, RgbColor, ConversationState, LightingMode},
//!     config::load_config,
//! };
//!
//! // Load configuration
//! let config = load_config("config/config.yaml")?;
//!
//! // Create bot state
//! let mut state = BotState::new();
//!
//! // Handle an event
//! let event = Event::WakeWordDetected;
//! if state.can_respond() {
//!     let _color = RgbColor::LISTENING;
//! }
//! # Ok::<(), anyhow::Error>(())
//! ```

// ============================================================================
// Module Declarations
// ============================================================================

pub mod commands;
pub mod config;
pub mod events;
pub mod state;

// ============================================================================
// Re-exports
// ============================================================================

pub use commands::Command;
pub use config::{load_config, load_default_config, SttConfig, SystemConfig};
pub use events::Event;
pub use state::{
    ActiveSubState, AmbientPattern, BotState, ConversationState, LightingMode, RgbColor,
};

// ============================================================================
// Prelude Module (Optional)
// ============================================================================
// A prelude module makes it easy to import commonly used types

// TODO: Optionally create a prelude module
// pub mod prelude {
//     pub use crate::{
//         Event, Command, BotState, ConversationState, LightingMode,
//         RgbColor, SystemConfig, load_config,
//     };
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_integration() {
        // Test that all modules work together
        let config = config::load_default_config().unwrap();
        let state = BotState::new();
        let event = Event::WakeWordDetected;
        let command = Command::SetColor(RgbColor::LISTENING);

        assert!(state.can_respond());
        assert!(event.is_audio_event());
        assert_eq!(config.audio.sample_rate, 16000);
        assert_eq!(command, Command::SetColor(RgbColor::new(255, 165, 0)));
    }

    #[test]
    fn test_event_to_state_flow() {
        // Test typical event flow: wake word → listening → thinking → speaking
        let mut state = BotState::new();
        assert_eq!(state.conversation_state, ConversationState::Ready);

        // Simulate wake word detection
        let wake_event = Event::WakeWordDetected;
        assert!(wake_event.is_audio_event());

        // Transition to listening
        state.conversation_state = ConversationState::Active(ActiveSubState::Listening);
        state.mark_interaction();
        assert!(state.can_respond());

        // Transition to thinking
        state.conversation_state = ConversationState::Active(ActiveSubState::Thinking);

        // Transition to speaking
        state.conversation_state = ConversationState::Active(ActiveSubState::Speaking);
    }

    #[test]
    fn test_command_serialization() {
        // Test that commands can be serialized/deserialized
        let cmd = Command::Speak {
            text: "Hello, world!".to_string(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }

    #[test]
    fn test_lighting_mode_patterns() {
        // Test that lighting modes work correctly
        let state_based = LightingMode::StateBased;
        let ambient = LightingMode::Ambient {
            pattern: AmbientPattern::Rainbow,
        };
        let minimal = LightingMode::Minimal;

        assert!(matches!(state_based, LightingMode::StateBased));
        assert!(matches!(ambient, LightingMode::Ambient { .. }));
        assert!(matches!(minimal, LightingMode::Minimal));
    }

    #[test]
    fn test_rgb_color_operations() {
        // Test color scaling and interpolation
        let red = RgbColor::RED;
        let half_red = red.scale(0.5);
        assert!(half_red.r < red.r);

        let blue = RgbColor::BLUE;
        let purple = red.lerp(&blue, 0.5);
        assert!(purple.r > 0 && purple.b > 0);
    }

    #[test]
    fn test_conversation_state_transitions() {
        // Test valid state transitions
        let mut state = BotState::new();

        // Ready → Observing
        state.conversation_state = ConversationState::Observing;
        assert!(state.can_respond());

        // Observing → Active (Listening)
        state.conversation_state = ConversationState::Active(ActiveSubState::Listening);

        // Silent mode (manual) - still responds but concisely
        state.conversation_state = ConversationState::Silent { manual: true };
        assert!(state.can_respond()); // Bot always can respond, even in Silent
    }

    #[test]
    fn test_status_led_commands() {
        // Test status LED pattern commands
        let green_solid = Command::SetGreenLeds(commands::StatusLedPattern::Solid);

        // Verify serialization
        let json = serde_json::to_string(&green_solid).unwrap();
        let deserialized: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(green_solid, deserialized);
    }

    #[test]
    fn test_presence_detection() {
        // Test PIR sensor event handling
        let presence = Event::PresenceDetected;
        assert!(presence.is_pir_event());

        use std::time::Duration;
        let no_presence = Event::NoPresenceSince(Duration::from_secs(300));
        assert!(no_presence.is_pir_event());
    }

    #[test]
    fn test_config_structure() {
        // Test that config has all necessary fields
        let config = config::load_default_config().unwrap();

        // GPIO config
        assert_eq!(config.gpio.pir_pin, 4);
        assert_eq!(config.gpio.rgb_pins.red, 12);
        assert_eq!(config.gpio.led_pins.green_1, 18);

        // Audio config
        assert!(config.audio.vosk.model_path.contains("vosk"));
        assert_eq!(config.audio.vosk.wake_phrase, "hey");

        // LLM config
        assert!(config.llm.model.contains("qwen"));

        // Behavior config
        assert!(config.behavior.conversation_timeout > 0);
    }
}
