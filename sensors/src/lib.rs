//! # sensors
//!
//! Hardware input controllers that emit Events.
//!
//! Phase 1 components:
//! - PIR sensor (motion detection)
//! - Audio controller (microphone input for STT)
//!
//! Future phases:
//! - Camera controller (visual presence detection)
//! - Ultrasonic sensors (proximity detection)
//! - DHT11 controller (temperature/humidity)

// ============================================================================
// Module Declarations
// ============================================================================

pub mod audio_controller;

// ============================================================================
// Re-exports
// ============================================================================

pub use audio_controller::{AudioController, AudioError};

// TODO: Phase 1.5 - Implement PirSensorController
// TODO: Phase 2 - Implement CameraController
// TODO: Phase 2 - Implement Dht11Controller

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
