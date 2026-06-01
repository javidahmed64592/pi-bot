//! # bot_core
//!
//! Core types and configuration for the Pi Bot companion system.
//!
//! This crate defines:
//! - Events: Sensor inputs (motion, wake word, speech, environment)
//! - Commands: Actuator outputs (LED colors, speech, display text)
//! - States: BotState with ConversationState and LightingMode
//! - Config: SystemConfig loaded from YAML

// TODO: Phase 1.1 - Define Event enum
// TODO: Phase 1.1 - Define Command enum
// TODO: Phase 1.2 - Define ConversationState enum
// TODO: Phase 1.2 - Define LightingMode enum
// TODO: Phase 1.2 - Define BotState struct
// TODO: Phase 1.3 - Define RgbColor struct with HSV utilities
// TODO: Phase 1.4 - Define SystemConfig struct and load_config()

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
