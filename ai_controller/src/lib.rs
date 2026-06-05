//! # ai_controller
//!
//! The brain of the Pi Bot companion system.
//!
//! Components:
//! - llm_service.rs: OpenAI-compatible API client for conversation (Phase 1.9) ✓
//! - memory_service.rs: JSON-based persistent memory system (Phase 1.10) ✓
//! - controller.rs: Main event loop, state machine, decision making (Phase 1.12) ✓

pub mod controller;
pub mod llm_service;
pub mod memory_service;

pub use controller::run_controller;
pub use llm_service::{LlmError, LlmService, Message};
pub use memory_service::{Exchange, MemoryService, Session};

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
