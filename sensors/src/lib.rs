//! Hardware Input Controllers
//!
//! Controllers that emit events from physical sensors.
//! All controllers follow event-driven architecture: read hardware → emit event.
//!
//! ## Phase 1 (Completed)
//! - `PirSensorController` - Motion detection with presence timeout
//! - `AudioController` - Microphone input for wake word and STT
//!
//! ## Future Phases
//! - Camera controller - Visual presence detection (Phase 2)
//! - DHT11 controller - Temperature/humidity (Phase 2)

// ============================================================================
// Module Declarations
// ============================================================================

pub mod audio_controller;
pub mod pir_sensor_controller;

// ============================================================================
// Re-exports
// ============================================================================

pub use audio_controller::{AudioController, AudioError};
pub use pir_sensor_controller::PirSensorController;

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
