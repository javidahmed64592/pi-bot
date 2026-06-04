//! # ai_controller
//!
//! The brain of the Pi Bot companion system.
//!
//! Components:
//! - llm_service.rs: Ollama API client for conversation (Phase 1.9)
//! - memory_service.rs: JSON-based persistent memory system (Phase 1.10)
//! - controller.rs: Main event loop, state machine, decision making (Phase 1.12)
//! - state_machine.rs: ConversationState transition logic (Phase 1.12)

pub mod llm_service;

pub use llm_service::{LlmError, LlmService, Message};

// TODO: Phase 1.10 - Implement memory_service.rs for basic JSON storage
// TODO: Phase 1.12 - Implement state_machine.rs
// TODO: Phase 1.12 - Implement controller.rs main loop

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
