//! # actuators
//!
//! Hardware output controllers that consume Commands.
//!
//! Phase 1 components:
//! - RGB LED controller (PWM-based color mixing, 3 GPIO pins)
//! - LED controller (LEDs for system state)
//! - Speaker controller (audio output via USB audio device)
//!
//! Future phases:
//! - LCD display controller (16x2 I2C display)

// ============================================================================
// Module Declarations
// ============================================================================

pub mod rgb_led_controller;
pub mod status_led_controller;

// ============================================================================
// Re-exports
// ============================================================================

pub use rgb_led_controller::RgbLedController;
pub use status_led_controller::StatusLedController;

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
