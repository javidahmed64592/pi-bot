//! Bot state types and color definitions
//!
//! This module defines:
//! - ConversationState: What the bot is doing (Ready, Active, etc.)
//! - LightingMode: How LEDs should behave (StateBased, Ambient, Minimal)
//! - BotState: The complete state of the bot system
//! - RgbColor: Color representation with utilities

use serde::{Deserialize, Serialize};
use std::time::Instant;

// ============================================================================
// Conversation States
// ============================================================================
// These determine WHAT THE BOT DOES (when it talks, when it's silent)

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConversationState {
    /// Bot is awake, monitoring sensors, ready to respond to wake phrase
    /// This is the default idle state
    Ready,

    /// Bot noticed something interesting and is deciding whether to speak
    /// Brief state (2-5 seconds) that may lead to Active or back to Ready
    Observing,

    /// Bot is actively engaged in conversation
    /// Has sub-states for Listening, Thinking, Speaking, Learning
    Active(ActiveSubState),

    /// Do Not Disturb mode - bot won't initiate conversations
    /// Still responds to wake phrase but stays concise
    Silent,
}

/// Sub-states when bot is in Active conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActiveSubState {
    /// Capturing speech from microphone
    Listening,

    /// Processing input through LLM
    Thinking,

    /// Storing important memory (brief)
    Learning,

    /// Playing TTS audio
    Speaking,
}

// ============================================================================
// Lighting Modes
// ============================================================================
// These determine WHAT YOU SEE (LED patterns, brightness)

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LightingMode {
    /// LED color/pattern reflects conversation state
    /// Default mode: Orange (listening), Blue (thinking), Green (speaking)
    StateBased,

    /// Decorative pattern independent of conversation
    /// Could be a gradient, rainbow, pulse, etc.
    /// Takes an optional pattern configuration
    Ambient { pattern: AmbientPattern },

    /// LED is very dim or completely off
    /// For meetings, sleep, or minimal distraction
    Minimal,
}

/// Ambient lighting pattern types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AmbientPattern {
    Gradient { colors: Vec<RgbColor> },
    Rainbow,
    Pulse { color: RgbColor },
    Static { color: RgbColor },
}

// ============================================================================
// Bot State
// ============================================================================
// The complete state of the bot system

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotState {
    /// Current conversation state (what bot is doing)
    pub conversation_state: ConversationState,

    /// Current lighting mode (what you see)
    pub lighting_mode: LightingMode,

    /// Is presence currently detected by PIR sensor?
    pub presence_detected: bool,

    /// When was the last user interaction?
    /// Used for idle timeouts and behavior decisions
    /// Note: This field is skipped during serialization since Instant is runtime-relative
    #[serde(skip, default = "Instant::now")]
    pub last_interaction: Instant,

    /// Current RGB LED brightness (0-100)
    pub brightness: u8,
    // Optional fields for Phase 2+:
    // pub current_emotion: Emotion,
    // pub system_health: HealthLevel,
    // pub ambient_pattern_index: usize,
}

impl BotState {
    /// Create a new BotState with default values
    pub fn new() -> Self {
        Self {
            conversation_state: ConversationState::Ready,
            lighting_mode: LightingMode::StateBased,
            presence_detected: false,
            last_interaction: Instant::now(),
            brightness: 50,
        }
    }

    /// Check if bot can respond to conversations
    /// Returns false if in Silent mode or system is unhealthy
    pub fn can_respond(&self) -> bool {
        !matches!(self.conversation_state, ConversationState::Silent)
    }

    /// Update last interaction time to now
    pub fn mark_interaction(&mut self) {
        self.last_interaction = Instant::now();
    }

    /// Get the color associated with the current conversation state
    pub fn state_color(&self) -> RgbColor {
        match &self.conversation_state {
            ConversationState::Ready => RgbColor::CYAN, // Calm, ready to respond
            ConversationState::Observing => RgbColor::YELLOW, // Alert, observing
            ConversationState::Active(sub) => match sub {
                ActiveSubState::Listening => RgbColor::LISTENING, // Orange
                ActiveSubState::Thinking => RgbColor::THINKING,   // Blue
                ActiveSubState::Speaking => RgbColor::SPEAKING,   // Green
                ActiveSubState::Learning => RgbColor::LEARNING,   // Purple
            },
            ConversationState::Silent => RgbColor::OFF, // DND mode, no light
        }
    }
}

impl Default for BotState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RGB Color
// ============================================================================
// Color representation with utility functions

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RgbColor {
    /// Red (0-255)
    pub r: u8,
    /// Green (0-255)
    pub g: u8,
    /// Blue (0-255)
    pub b: u8,
}

impl RgbColor {
    /// Create a new RGB color
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    // ========================================================================
    // Predefined Colors
    // ========================================================================
    pub const RED: Self = Self::new(255, 0, 0);
    pub const GREEN: Self = Self::new(0, 255, 0);
    pub const BLUE: Self = Self::new(0, 0, 255);
    pub const ORANGE: Self = Self::new(255, 165, 0);
    pub const PURPLE: Self = Self::new(128, 0, 128);
    pub const YELLOW: Self = Self::new(255, 255, 0);
    pub const CYAN: Self = Self::new(0, 255, 255);
    pub const WHITE: Self = Self::new(255, 255, 255);
    pub const OFF: Self = Self::new(0, 0, 0);

    // Conversation state colors (from docs)
    pub const LISTENING: Self = Self::ORANGE;
    pub const THINKING: Self = Self::BLUE;
    pub const SPEAKING: Self = Self::GREEN;
    pub const LEARNING: Self = Self::PURPLE;

    /// Scale brightness (0.0 = off, 1.0 = full)
    pub fn scale(&self, brightness: f32) -> Self {
        Self {
            r: (self.r as f32 * brightness) as u8,
            g: (self.g as f32 * brightness) as u8,
            b: (self.b as f32 * brightness) as u8,
        }
    }

    /// Linear interpolation between two colors
    /// t: 0.0 = this color, 1.0 = other color
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: (self.r as f32 + (other.r as f32 - self.r as f32) * t) as u8,
            g: (self.g as f32 + (other.g as f32 - self.g as f32) * t) as u8,
            b: (self.b as f32 + (other.b as f32 - self.b as f32) * t) as u8,
        }
    }

    // Convert from HSV color space
    // Useful for creating rainbow effects and gradients
    // h: hue (0.0-360.0), s: saturation (0.0-1.0), v: value/brightness (0.0-1.0)
    // TODO: Implement this for advanced color effects (Phase 2)
    // pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
    //     // HSV to RGB conversion algorithm
    // }
}

impl From<(u8, u8, u8)> for RgbColor {
    fn from(tuple: (u8, u8, u8)) -> Self {
        Self::new(tuple.0, tuple.1, tuple.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_state_creation() {
        let state = BotState::new();
        assert_eq!(state.conversation_state, ConversationState::Ready);
        assert_eq!(state.lighting_mode, LightingMode::StateBased);
        assert!(!state.presence_detected);
        assert_eq!(state.brightness, 50);
        assert!(state.can_respond());

        let now = state.last_interaction;
        let mut state = state;
        state.mark_interaction();
        assert!(state.last_interaction >= now);
    }

    #[test]
    fn test_serde_serialization() {
        let bot_state = BotState::new();
        let json = serde_json::to_string(&bot_state).unwrap();
        let parsed: BotState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.conversation_state, ConversationState::Ready);
    }

    #[test]
    fn test_state_colors() {
        let mut state = BotState::new();

        // Ready state
        assert_eq!(state.state_color(), RgbColor::CYAN);

        // Observing state
        state.conversation_state = ConversationState::Observing;
        assert_eq!(state.state_color(), RgbColor::YELLOW);

        // Active states
        state.conversation_state = ConversationState::Active(ActiveSubState::Listening);
        assert_eq!(state.state_color(), RgbColor::LISTENING);

        state.conversation_state = ConversationState::Active(ActiveSubState::Thinking);
        assert_eq!(state.state_color(), RgbColor::THINKING);

        state.conversation_state = ConversationState::Active(ActiveSubState::Speaking);
        assert_eq!(state.state_color(), RgbColor::SPEAKING);

        state.conversation_state = ConversationState::Active(ActiveSubState::Learning);
        assert_eq!(state.state_color(), RgbColor::LEARNING);

        // Silent state
        state.conversation_state = ConversationState::Silent;
        assert_eq!(state.state_color(), RgbColor::OFF);
    }
}
