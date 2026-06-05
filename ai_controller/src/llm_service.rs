//! # LLM Service
//!
//! OpenAI-compatible API client for conversational AI using llamafile.
//!
//! ## Architecture
//!
//! LlmService handles:
//! - Building conversation context from system prompt and message history
//! - Making API calls to local llamafile server (OpenAI-compatible)
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
//!     model: "qwen2.5-3b-instruct".to_string(),
//!     api_host: "http://localhost:8080".to_string(),
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
    #[error("LLM API request failed: {0}")]
    ApiRequestError(String),

    #[error("Failed to parse LLM response: {0}")]
    ResponseParseError(String),

    #[error("LLM returned empty response")]
    EmptyResponse,

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Context too long: {current} tokens exceeds limit of {max}")]
    ContextTooLong { current: usize, max: usize },

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

// ============================================================================
// API Types (OpenAI-compatible JSON Schema)
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

/// Request to OpenAI-compatible /v1/chat/completions endpoint
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// Response from OpenAI-compatible /v1/chat/completions endpoint (non-streaming)
#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[allow(dead_code)]
    id: String,

    #[allow(dead_code)]
    object: String,

    #[allow(dead_code)]
    created: u64,

    #[allow(dead_code)]
    model: String,

    choices: Vec<ChatChoice>,

    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    #[allow(dead_code)]
    index: u32,
    message: Message,

    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// ============================================================================
// LlmService - Conversation AI
// ============================================================================

/// LLM service for generating conversational responses
///
/// Uses OpenAI-compatible API to communicate with locally-running LLM (llamafile).
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
    /// Result containing LlmService or error if API server is not available
    pub async fn new(config: LlmConfig) -> Result<Self, LlmError> {
        // Validate config
        if config.model.is_empty() {
            return Err(LlmError::InvalidConfig("model cannot be empty".to_string()));
        }

        if config.api_host.is_empty() {
            return Err(LlmError::InvalidConfig(
                "api_host cannot be empty".to_string(),
            ));
        }

        // Create HTTP client with longer timeout for Pi 5 generation
        // Smaller models (3B) typically respond in 5-30 seconds on Pi
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(180)) // 3 minutes
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| {
                LlmError::ApiRequestError(format!("Failed to create HTTP client: {}", e))
            })?;

        info!(
            "Initialized LLM service with model: {} (host: {})",
            config.model, config.api_host
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
    /// - Sends request to OpenAI-compatible API
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

        // Build request (OpenAI format)
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            stream: Some(false), // Non-streaming for simplicity in Phase 1
            temperature: Some(self.config.temperature),
            max_tokens: Some(self.config.max_context_length),
        };

        // Send request to OpenAI-compatible endpoint
        let url = format!("{}/v1/chat/completions", self.config.api_host);
        debug!("Sending request to: {}", url);
        debug!("Request: {:?}", request);

        let start_time = std::time::Instant::now();

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
                        "Request timed out after 180s. Model may be too slow on this hardware."
                            .to_string(),
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

        let duration = start_time.elapsed();

        // Log metrics
        if let Some(usage) = chat_response.usage {
            let duration_secs = duration.as_secs_f64();
            let tokens_per_sec = usage.completion_tokens as f64 / duration_secs;
            info!(
                "Generated response: {} tokens in {:.1}s ({:.1} tokens/s)",
                usage.completion_tokens, duration_secs, tokens_per_sec
            );
            debug!(
                "Prompt tokens: {}, Completion tokens: {}, Total: {}",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
            );
        } else {
            info!("Generated response in {:.1}s", duration.as_secs_f64());
        }

        // Extract response from first choice
        let response_text = chat_response
            .choices
            .first()
            .map(|choice| choice.message.content.trim().to_string())
            .ok_or(LlmError::EmptyResponse)?;

        if response_text.is_empty() {
            return Err(LlmError::EmptyResponse);
        }

        info!("Response: '{}'", response_text);

        Ok(response_text)
    }

    /// Check if API server is available
    ///
    /// # Returns
    ///
    /// Result indicating if server is reachable
    pub async fn check_health(&self) -> Result<bool, LlmError> {
        let url = format!("{}/v1/models", self.config.api_host);
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
    #[ignore] // Requires llamafile to be running
    async fn test_llm_service_init() {
        let config = LlmConfig {
            model: "qwen2.5-3b-instruct".to_string(),
            api_host: "http://localhost:8080".to_string(),
            temperature: 0.8,
            max_context_length: 4096,
            system_prompt: "You are a test assistant.".to_string(),
        };

        let result = LlmService::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires llamafile to be running with model loaded
    async fn test_generate() {
        let config = LlmConfig {
            model: "qwen2.5-3b-instruct".to_string(),
            api_host: "http://localhost:8080".to_string(),
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
