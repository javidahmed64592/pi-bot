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

    /// LED pattern configurations
    pub led_patterns: LedPatternConfig,
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
    // LCD I2C address (Phase 2+)
    // pub lcd_i2c_address: u8,
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

/// Ultrasonic sensor pin configuration (trigger and echo)
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

    /// Path to long-term memory JSON file
    pub long_term_storage: String,

    /// Maximum number of recent messages to keep in short-term memory
    pub max_short_term: usize,

    /// Enable automatic fact extraction from conversations
    pub fact_extraction_enabled: bool,
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
// LED Pattern Configuration
// ============================================================================

/// LED pattern names for different states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedPatternConfig {
    /// Pattern for idle/ready state
    pub idle_ambient: String,

    /// Pattern for listening state
    pub listening: String,

    /// Pattern for thinking state
    pub thinking: String,

    /// Pattern for speaking state
    pub speaking: String,
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
