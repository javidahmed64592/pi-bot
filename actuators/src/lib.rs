//! Hardware Output Controllers
//!
//! Controllers that consume commands and control physical hardware.
//! All controllers follow command-driven architecture: receive command → control hardware.
//!
//! ## Phase 1 (Completed)
//! - `RgbLedController` - PWM-based RGB LED with color patterns
//! - `StatusLedController` - Multi-LED status indicators with brightness control
//! - `SpeakerController` - Audio playback via Piper TTS
//!
//! ## Future Phases
//! - LCD display controller - 16x2 I2C text display (Phase 2)

// ============================================================================
// Module Declarations
// ============================================================================

pub mod rgb_led_controller;
pub mod speaker_controller;
pub mod status_led_controller;

// ============================================================================
// Re-exports
// ============================================================================

pub use rgb_led_controller::RgbLedController;
pub use speaker_controller::SpeakerController;
pub use status_led_controller::StatusLedController;

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
