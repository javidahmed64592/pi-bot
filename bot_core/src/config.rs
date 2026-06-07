//! Configuration loading from YAML files
//!
//! This module defines the SystemConfig struct and load_config() function.
//! Configuration is loaded from config/config.yaml at runtime.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;

// ============================================================================
// Main Configuration Struct
// ============================================================================

/// Complete system configuration loaded from config.yaml
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemConfig {
    /// GPIO pin mappings for all hardware
    pub gpio: GpioConfig,

    /// Audio configuration (Vosk, Piper, microphone settings)
    pub audio: AudioConfig,

    /// LLM configuration (Ollama, model, prompts)
    pub llm: LlmConfig,

    /// Memory system configuration
    pub memory: MemoryConfig,

    /// Bot behavior parameters
    pub behavior: BehaviorConfig,

    /// RGB LED configuration for conversation states
    pub rgb_led: RgbLedConfig,

    /// Status LED pattern configuration
    pub status_leds: StatusLedConfig,
}

// ============================================================================
// GPIO Configuration
// ============================================================================

/// GPIO pin assignments for all hardware components
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpioConfig {
    // ========================================================================
    // Sensors (inputs)
    // ========================================================================
    /// PIR motion sensor pin
    pub pir_pin: u8,

    // DHT11 temperature/humidity sensor pin (Phase 2+)
    // pub dht11_pin: u8,

    // ========================================================================
    // Actuators (outputs)
    // ========================================================================
    /// RGB LED pins (red, green, blue)
    pub rgb_pins: RgbPins,

    /// Status LED pins (green and red indicators)
    pub led_pins: LedPins,

    /// LCD I2C address
    pub lcd_i2c_address: u8,
}

/// RGB LED pin configuration (3 pins for PWM)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RgbPins {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// LED status indicator pin configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedPins {
    pub green_1: u8,
    pub green_2: u8,
    pub red_1: u8,
    pub red_2: u8,
}

// Ultrasonic sensor pin configuration (trigger and echo)
// TODO: Implement this struct for Phase 2
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct UltrasonicPins {
//     pub trigger: u8,
//     pub echo: u8,
// }

// ============================================================================
// Audio Configuration
// ============================================================================

/// Audio system configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Microphone device name (e.g., "default")
    pub microphone_device: String,

    /// Speaker device name (e.g., "default")
    pub speaker_device: String,

    /// Audio sample rate (typically 16000 Hz for Vosk)
    pub sample_rate: u32,

    /// Vosk configuration
    pub vosk: VoskConfig,

    /// Piper TTS configuration
    pub piper: PiperConfig,
}

/// Vosk wake word and STT configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoskConfig {
    /// Path to Vosk model directory
    pub model_path: String,

    /// Wake phrase to listen for (e.g., "hey bot")
    pub wake_phrase: String,

    /// Use keyword spotting mode (faster, less CPU)
    pub keyword_mode: bool,

    /// Speech-to-text capture settings
    pub stt: SttConfig,
}

/// Speech-to-text capture configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SttConfig {
    /// Maximum duration to capture speech after wake word (seconds)
    pub capture_timeout: f32,

    /// Silence duration to end speech capture (seconds)
    pub silence_threshold: f32,

    /// Minimum speech duration to process (seconds)
    pub min_speech_duration: f32,
}

/// Piper TTS configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PiperConfig {
    /// Voice name (e.g., "en_US-lessac-medium")
    pub voice: String,

    /// Path to Piper model file
    pub model_path: String,

    /// Path to Piper config file
    pub config_path: String,
}

// ============================================================================
// LLM Configuration
// ============================================================================

/// Ollama LLM configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Model name (e.g., "qwen2.5:7b-instruct")
    pub model: String,

    /// Ollama API host URL
    pub ollama_host: String,

    /// Temperature for generation (0.0-2.0)
    pub temperature: f32,

    /// Maximum context length in tokens
    pub max_context_length: u32,

    /// System prompt that defines bot personality
    pub system_prompt: String,
}

// ============================================================================
// Memory Configuration
// ============================================================================

/// Memory system configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Directory for session storage
    pub session_storage: String,

    /// Directory for long-term memory (facts database)
    pub long_term_storage: String,

    /// Maximum number of recent messages to keep in short-term memory
    pub max_short_term: usize,

    /// Enable automatic fact extraction from conversations
    pub fact_extraction_enabled: bool,

    /// Embeddings configuration for semantic search (Phase 2.5)
    #[serde(default)]
    pub embeddings: Option<EmbeddingsConfig>,

    /// Search configuration for semantic memory
    #[serde(default = "default_search_config")]
    pub search: SearchConfig,
}

/// Embeddings model configuration for semantic search
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    /// Path to ONNX model file
    pub model_path: String,

    /// Path to tokenizer JSON file
    pub tokenizer_path: String,

    /// Embedding dimensions (384 for all-MiniLM-L6-v2)
    pub dimensions: usize,

    /// Enable embeddings (set to false to disable semantic memory)
    pub enabled: bool,
}

/// Search configuration for semantic memory
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Number of top facts to retrieve
    pub top_k: usize,

    /// Minimum cosine similarity threshold (0.0-1.0)
    pub min_similarity: f32,

    /// Maximum number of facts to store
    pub max_facts: usize,
}

fn default_search_config() -> SearchConfig {
    SearchConfig {
        top_k: 5,
        min_similarity: 0.7,
        max_facts: 1000,
    }
}

// ============================================================================
// Behavior Configuration
// ============================================================================

/// Bot behavior parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorConfig {
    /// Range for passive observation interval in seconds [min, max]
    /// Bot randomly picks a time in this range to check in
    pub passive_observation_interval: [u64; 2],

    /// Conversation timeout in seconds (silence before ending conversation)
    pub conversation_timeout: u64,

    /// Idle timeout in seconds (return to low-power mode)
    pub idle_timeout: u64,

    /// Default Do Not Disturb duration in seconds
    pub do_not_disturb_duration: u64,
}

impl BehaviorConfig {
    /// Get a random observation interval within the configured range
    pub fn random_observation_interval(&self) -> Duration {
        let secs = rand::random_range(
            self.passive_observation_interval[0]..=self.passive_observation_interval[1],
        );
        Duration::from_secs(secs)
    }
}

// ============================================================================
// LED Configuration
// ============================================================================

/// RGB LED configuration for all conversation states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RgbLedConfig {
    /// Ready state (default, monitoring)
    pub ready: StateAppearance,

    /// Observing state (deciding whether to speak)
    pub observing: StateAppearance,

    /// Silent state (Do Not Disturb)
    pub silent: StateAppearance,

    /// Active conversation sub-states
    pub active: ActiveStateConfig,
}

impl RgbLedConfig {
    /// Get the appearance (pattern + color) for a given conversation state
    ///
    /// # Example
    /// ```no_run
    /// use bot_core::config::load_config;
    /// use bot_core::state::ConversationState;
    ///
    /// let config = load_config("config/config.yaml")?;
    /// let state = ConversationState::Ready;
    ///
    /// // Get the configured appearance for this state
    /// let appearance = config.rgb_led.get_appearance(&state);
    ///
    /// // Convert to RgbColor for LED control
    /// let color = appearance.to_rgb_color();
    /// println!("Ready state: {} pattern with color RGB({}, {}, {})",
    ///          appearance.pattern, color.r, color.g, color.b);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn get_appearance(&self, state: &crate::state::ConversationState) -> &StateAppearance {
        use crate::state::{ActiveSubState, ConversationState};

        match state {
            ConversationState::Ready => &self.ready,
            ConversationState::Observing => &self.observing,
            ConversationState::Silent { .. } => &self.silent,
            ConversationState::Active(sub) => match sub {
                ActiveSubState::Listening => &self.active.listening,
                ActiveSubState::Thinking => &self.active.thinking,
                ActiveSubState::Speaking => &self.active.speaking,
                ActiveSubState::Learning => &self.active.learning,
            },
        }
    }
}

/// Active conversation sub-state appearance config
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveStateConfig {
    pub listening: StateAppearance,
    pub thinking: StateAppearance,
    pub speaking: StateAppearance,
    pub learning: StateAppearance,
}

/// Pattern and color configuration for a single state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateAppearance {
    /// Pattern name ("breathing", "pulse", "solid", "gradient", "rainbow")
    pub pattern: String,

    /// RGB color as [R, G, B] array (0-255 for each channel)
    pub color: [u8; 3],
}

impl StateAppearance {
    /// Convert the color array to an RgbColor struct
    pub fn to_rgb_color(&self) -> crate::state::RgbColor {
        crate::state::RgbColor::new(self.color[0], self.color[1], self.color[2])
    }

    /// Convert the pattern string to LedPattern enum
    pub fn pattern_to_led_pattern(&self) -> crate::commands::LedPattern {
        use crate::commands::LedPattern;

        match self.pattern.to_lowercase().as_str() {
            "solid" => LedPattern::Solid,
            "breathing" => LedPattern::Breathing,
            "pulse" => LedPattern::Pulse,
            "gradient" => LedPattern::Gradient,
            "rainbow" => LedPattern::Rainbow,
            "colorcycle" | "color_cycle" => LedPattern::ColorCycle,
            _ => {
                log::warn!(
                    "Unknown LED pattern '{}', defaulting to Solid",
                    self.pattern
                );
                LedPattern::Solid
            }
        }
    }
}

/// Status LED pattern configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusLedConfig {
    /// Green LED patterns for active states
    pub green: GreenLedPatterns,

    /// Red LED patterns for inactive/error states
    pub red: RedLedPatterns,
}

/// Green status LED patterns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GreenLedPatterns {
    /// Pattern when bot is ready ("solid")
    pub ready: String,

    /// Pattern when bot is active ("breathing")
    pub active: String,
}

/// Red status LED patterns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedLedPatterns {
    /// Pattern for Silent/DND mode ("breathing")
    pub silent: String,

    /// Pattern for system errors ("flashing")
    pub error: String,
}

// ============================================================================
// Configuration Loading
// ============================================================================

/// Load configuration from a YAML file
///
/// # Arguments
/// * `path` - Path to the config.yaml file (e.g., "config/config.yaml")
///
/// # Returns
/// * `Result<SystemConfig>` - Loaded configuration or error
///
/// # Example
/// ```no_run
/// use bot_core::config::load_config;
///
/// let config = load_config("config/config.yaml")?;
/// println!("Loaded config for model: {}", config.llm.model);
/// # Ok::<(), anyhow::Error>(())
/// ```
// TODO: Implement this function
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<SystemConfig> {
    let contents = fs::read_to_string(path.as_ref()).context("Failed to read config file")?;

    // Rust Tip: Parse YAML into struct
    let config: SystemConfig =
        serde_yaml::from_str(&contents).context("Failed to parse config YAML")?;

    Ok(config)
}

/// Load configuration from default location (config/config.yaml)
///
/// This function intelligently locates the config file:
/// - First tries the relative path "config/config.yaml" (for production)
/// - Falls back to workspace root using CARGO_MANIFEST_DIR (for tests)
pub fn load_default_config() -> Result<SystemConfig> {
    let relative_path = Path::new("config/config.yaml");

    // Try relative path first (works when running from workspace root)
    if relative_path.exists() {
        return load_config(relative_path);
    }

    // Fall back to workspace root (for tests and other scenarios)
    // CARGO_MANIFEST_DIR points to bot_core/, so go up one level to workspace root
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("Failed to find workspace root")?;
    let config_path = workspace_root.join("config/config.yaml");

    load_config(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_loading() {
        let config = load_default_config().unwrap();
        assert_eq!(config.audio.sample_rate, 16000);
    }

    #[test]
    fn test_config_serialization() {
        let config = load_default_config().unwrap();
        let serialized = serde_yaml::to_string(&config).unwrap();
        let deserialized: SystemConfig = serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
