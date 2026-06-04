//! Commands sent to actuators and system components
//!
//! Commands flow from controller → actuators. They represent "do this thing"
//! to control hardware or change system state.

use crate::state::{ConversationState, LightingMode, RgbColor};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    // ============================================================================
    // Visual Actuators - RGB LED
    // ============================================================================
    /// Set RGB LED to a solid color
    SetColor(RgbColor),

    /// Set RGB LED to display a pattern
    /// pattern: The pattern type (Solid, Breathing, Pulse, Gradient, etc.)
    /// colors: Vector of colors to use in the pattern
    SetPattern {
        pattern: LedPattern,
        colors: Vec<RgbColor>,
    },

    /// Turn off the RGB LED
    LedOff,

    // ============================================================================
    // Visual Actuators - Status LEDs
    // ============================================================================
    // Green LEDs = active state indicators, Red LEDs = idle/error state indicators
    // Only one set (green OR red) should be active at any time
    /// Set green LED pattern (active state indicator)
    /// Patterns: Solid (ready), Breathing (processing), Off
    SetGreenLeds(StatusLedPattern),

    /// Set red LED pattern (idle/error state indicator)
    /// Patterns: Breathing (DND/idle), Flashing (error), Off
    SetRedLeds(StatusLedPattern),

    // ============================================================================
    // Visual Actuators - Display (Phase 2+)
    // ============================================================================
    // Show text on the LCD display (16x2 characters)
    // line1: Text for first line (max 16 chars)
    // line2: Text for second line (max 16 chars)
    // TODO: Add this variant with named fields
    // ShowText { line1: String, line2: String },

    // Clear the display
    // TODO: Add this variant
    // ClearDisplay,

    // ============================================================================
    // Audio Actuators - Speaker/TTS
    // ============================================================================
    /// Speak the given text using Piper TTS
    /// text: What to say
    Speak { text: String },

    // emotion: Tone/emotion to use (optional, for future voice modulation)
    // Or with emotion: Speak { text: String, emotion: Emotion },
    /// Stop currently playing speech
    StopSpeaking,

    // ============================================================================
    // System State Commands
    // ============================================================================
    /// Lock the bot (enter Silent mode, activate red LEDs)
    LockBot,

    /// Unlock the bot (exit Silent mode, activate green LEDs)
    UnlockBot,

    /// Tell audio pipeline to start listening for speech
    /// (after wake word detected)
    StartListening,

    /// Tell audio pipeline to stop listening
    StopListening,

    /// Transition to a new conversation state
    EnterConversationState(ConversationState),

    /// Change the lighting mode
    SetLightingMode(LightingMode),
}

// ============================================================================
// Supporting Types
// ============================================================================

/// LED pattern types for RGB LED
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LedPattern {
    Solid,      // Static color
    Breathing,  // Smooth fade in/out
    Pulse,      // Quick pulse animation
    Gradient,   // Transition between colors
    Rainbow,    // Cycle through rainbow
    ColorCycle, // Cycle through provided colors
}

/// Status LED pattern types for green/red LEDs
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StatusLedPattern {
    Solid,     // Constant on (green: ready)
    Breathing, // Smooth fade in/out (green: processing, red: DND)
    Flashing,  // Quick on/off (red: error)
    Off,       // LEDs off
}

/// Emotion/tone for speech (future feature)
// TODO: Define this enum if you want emotion support
// #[derive(Debug, Clone, Copy)]
// pub enum Emotion {
//     Neutral,
//     Happy,
//     Sad,
//     Excited,
//     Calm,
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_serialization() {
        let cmd = Command::SetPattern {
            pattern: LedPattern::Gradient,
            colors: vec![
                RgbColor { r: 255, g: 0, b: 0 },
                RgbColor { r: 0, g: 255, b: 0 },
                RgbColor { r: 0, g: 0, b: 255 },
            ],
        };

        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized_cmd: Command = serde_json::from_str(&json).unwrap();

        assert_eq!(cmd, deserialized_cmd);
    }
}
