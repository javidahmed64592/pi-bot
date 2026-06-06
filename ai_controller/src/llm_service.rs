//! # LLM Service
//!
//! Ollama API client for conversational AI using Qwen2.5 7B.
//!
//! ## Architecture
//!
//! LlmService handles:
//! - Building conversation context from system prompt and message history
//! - Making API calls to local Ollama server
//! - Streaming responses for real-time feedback
//! - Error handling and retry logic
//!
//! ## Usage
//!
//! ```ignore
//! use ai_controller::LlmService;
//! use bot_core::config::LlmConfig;
//!
//! # async fn example() {
//! let config = LlmConfig {
//!     model: "qwen2.5:7b-instruct".to_string(),
//!     ollama_host: "http://localhost:11434".to_string(),
//!     temperature: 0.8,
//!     max_context_length: 4096,
//!     system_prompt: "You are a helpful assistant.".to_string(),
//! };
//!
//! let service = LlmService::new(config).await.expect("Failed to init LLM");
//!
//! let response = service.generate("Hello, how are you?", &[]).await.expect("Failed to generate");
//! println!("Bot: {}", response);
//! # }
//! ```

use anyhow::Result;
use bot_core::config::LlmConfig;
use log::{debug, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Ollama API request failed: {0}")]
    ApiRequestError(String),

    #[error("Failed to parse Ollama response: {0}")]
    ResponseParseError(String),

    #[error("Ollama returned empty response")]
    EmptyResponse,

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Context too long: {current} tokens exceeds limit of {max}")]
    ContextTooLong { current: usize, max: usize },

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

// ============================================================================
// API Types (Ollama JSON Schema)
// ============================================================================

/// Message in conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role: "system", "user", or "assistant"
    pub role: String,

    /// Message content
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Request to Ollama /api/chat endpoint
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    options: ChatOptions,
}

/// Generation options for Ollama
#[derive(Debug, Serialize)]
struct ChatOptions {
    temperature: f32,
    num_ctx: u32, // context window size
}

/// Response from Ollama /api/chat endpoint (non-streaming)
#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: Message,

    #[allow(dead_code)]
    done: bool,

    #[serde(default)]
    total_duration: Option<u64>,

    #[serde(default)]
    prompt_eval_count: Option<u32>,

    #[serde(default)]
    eval_count: Option<u32>,
}

// ============================================================================
// LlmService - Conversation AI
// ============================================================================

/// LLM service for generating conversational responses
///
/// Uses Ollama API to communicate with locally-running LLM (Qwen2.5 7B).
/// Manages conversation context and generates contextually appropriate responses.
pub struct LlmService {
    config: LlmConfig,
    client: Client,
}

impl LlmService {
    /// Create a new LLM service
    ///
    /// # Arguments
    ///
    /// * `config` - LLM configuration (model name, host, temperature, etc.)
    ///
    /// # Returns
    ///
    /// Result containing LlmService or error if Ollama is not available
    pub async fn new(config: LlmConfig) -> Result<Self, LlmError> {
        // Validate config
        if config.model.is_empty() {
            return Err(LlmError::InvalidConfig("model cannot be empty".to_string()));
        }

        if config.ollama_host.is_empty() {
            return Err(LlmError::InvalidConfig(
                "ollama_host cannot be empty".to_string(),
            ));
        }

        // Create HTTP client with longer timeout for Pi 5 generation
        // Qwen 2.5 7B can take 30-90 seconds for first response on Pi
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(180)) // 3 minutes
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| {
                LlmError::ApiRequestError(format!("Failed to create HTTP client: {}", e))
            })?;

        info!(
            "Initialized LLM service with model: {} (host: {})",
            config.model, config.ollama_host
        );

        Ok(Self { config, client })
    }

    /// Generate a response to user input
    ///
    /// # Arguments
    ///
    /// * `user_input` - User's message text
    /// * `history` - Recent conversation history (oldest to newest)
    ///
    /// # Returns
    ///
    /// Result containing assistant's response text or LlmError
    ///
    /// # Behavior
    ///
    /// - Builds conversation context: [system prompt, history..., user input]
    /// - Sends request to Ollama API
    /// - Returns complete response text
    /// - Logs generation metrics (tokens, duration)
    pub async fn generate(
        &self,
        user_input: &str,
        history: &[Message],
    ) -> Result<String, LlmError> {
        if user_input.is_empty() {
            warn!("Attempted to generate response for empty input");
            return Err(LlmError::EmptyResponse);
        }

        debug!("Generating response for: '{}'", user_input);

        // Build conversation context
        let mut messages = Vec::new();

        // 1. System prompt (personality)
        messages.push(Message::system(&self.config.system_prompt));

        // 2. Recent conversation history
        messages.extend(history.iter().cloned());

        // 3. Current user input
        messages.push(Message::user(user_input));

        // Check context length (rough estimate: ~4 chars per token)
        let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();
        let estimated_tokens = total_chars / 4;

        if estimated_tokens > self.config.max_context_length as usize {
            return Err(LlmError::ContextTooLong {
                current: estimated_tokens,
                max: self.config.max_context_length as usize,
            });
        }

        // Build request
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            stream: false, // Non-streaming for simplicity in Phase 1
            options: ChatOptions {
                temperature: self.config.temperature,
                num_ctx: self.config.max_context_length,
            },
        };

        // Send request to Ollama
        let url = format!("{}/api/chat", self.config.ollama_host);
        debug!("Sending request to: {}", url);
        debug!("Request: {:?}", request);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                // Provide more context on timeout errors
                let error_msg = e.to_string();
                if error_msg.contains("timeout") {
                    LlmError::ApiRequestError(
                        "Request timed out after 180s. Model may be too slow on this hardware. Consider using a smaller model.".to_string()
                    )
                } else {
                    LlmError::ApiRequestError(format!("HTTP request failed: {}", error_msg))
                }
            })?;

        // Check status code
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiRequestError(format!(
                "HTTP {} - {}",
                status, error_text
            )));
        }

        // Parse response
        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ResponseParseError(format!("JSON parse failed: {}", e)))?;

        // Log metrics
        if let (Some(duration), Some(prompt_tokens), Some(completion_tokens)) = (
            chat_response.total_duration,
            chat_response.prompt_eval_count,
            chat_response.eval_count,
        ) {
            let duration_secs = duration as f64 / 1_000_000_000.0;
            let tokens_per_sec = completion_tokens as f64 / duration_secs;
            info!(
                "Generated response: {} tokens in {:.1}s ({:.1} tokens/s)",
                completion_tokens, duration_secs, tokens_per_sec
            );
            debug!(
                "Prompt tokens: {}, Completion tokens: {}",
                prompt_tokens, completion_tokens
            );
        }

        let response_text = chat_response.message.content.trim().to_string();

        if response_text.is_empty() {
            return Err(LlmError::EmptyResponse);
        }

        info!("Response: '{}'", response_text);

        Ok(response_text)
    }

    /// Check if Ollama server is available
    ///
    /// # Returns
    ///
    /// Result indicating if server is reachable
    pub async fn check_health(&self) -> Result<bool, LlmError> {
        let url = format!("{}/api/tags", self.config.ollama_host);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| LlmError::ApiRequestError(format!("Health check failed: {}", e)))?;

        Ok(response.status().is_success())
    }

    /// Get the model name
    pub fn model_name(&self) -> &str {
        &self.config.model
    }

    /// Get the configured temperature
    pub fn temperature(&self) -> f32 {
        self.config.temperature
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_constructors() {
        let system = Message::system("You are helpful");
        assert_eq!(system.role, "system");
        assert_eq!(system.content, "You are helpful");

        let user = Message::user("Hello");
        assert_eq!(user.role, "user");
        assert_eq!(user.content, "Hello");

        let assistant = Message::assistant("Hi there");
        assert_eq!(assistant.role, "assistant");
        assert_eq!(assistant.content, "Hi there");
    }

    #[tokio::test]
    #[ignore] // Requires Ollama to be running
    async fn test_llm_service_init() {
        let config = LlmConfig {
            model: "qwen2.5:7b-instruct".to_string(),
            ollama_host: "http://localhost:11434".to_string(),
            temperature: 0.8,
            max_context_length: 4096,
            system_prompt: "You are a test assistant.".to_string(),
        };

        let result = LlmService::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires Ollama to be running with model loaded
    async fn test_generate() {
        let config = LlmConfig {
            model: "qwen2.5:7b-instruct".to_string(),
            ollama_host: "http://localhost:11434".to_string(),
            temperature: 0.8,
            max_context_length: 4096,
            system_prompt: "You are a helpful assistant. Keep responses brief.".to_string(),
        };

        let service = LlmService::new(config)
            .await
            .expect("Failed to init service");
        let response = service
            .generate("What is 2+2?", &[])
            .await
            .expect("Failed to generate");

        assert!(!response.is_empty());
        assert!(response.contains("4") || response.contains("four"));
    }
}
