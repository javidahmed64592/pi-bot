//! # ai_controller
//!
//! The brain of the Pi Bot companion system.
//!
//! Components:
//! - llm_service.rs: Ollama API client for conversation (Phase 1.9) ✓
//! - memory_service.rs: JSON-based persistent memory system (Phase 1.10) ✓
//! - embedding_service.rs: ONNX-based text embeddings for semantic search (Phase 2.5)
//! - observation_mode.rs: Passive observation and proactive conversation initiation (Phase 2.4) ✓
//! - controller.rs: Main event loop, state machine, decision making (Phase 1.12) ✓

pub mod controller;
pub mod embedding_service;
pub mod llm_service;
pub mod memory_service;
pub mod observation_mode;

pub use controller::run_controller;
pub use embedding_service::{cosine_similarity, EmbeddingService};
pub use llm_service::{LlmError, LlmService, Message};
pub use memory_service::{Exchange, Fact, FactSource, MemoryService, Session};
pub use observation_mode::ObservationContext;

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
