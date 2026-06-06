//! Events emitted by sensors and system components
//!
//! Events flow from sensors → controller. They represent "something happened"
//! in the physical world or system.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    // ============================================================================
    // PIR Sensor Events
    // ============================================================================
    /// PIR sensor detected motion/presence in the room
    PresenceDetected,

    /// No presence has been detected for the given duration
    /// Used to trigger presence timeout behaviors
    NoPresenceSince(Duration),

    // ============================================================================
    // Audio Sensor Events (Microphone)
    // ============================================================================
    /// Wake word ("Hey Bot") was detected by Vosk
    /// This triggers transition from Ready → Active state
    WakeWordDetected,

    /// Speech capture has started (after wake word detected)
    /// Indicates the system is actively listening and transcribing user speech
    SpeechCaptureStarted,

    /// Speech was captured and transcribed by Vosk
    /// The String contains the transcribed text from the user
    SpeechCaptured(String),

    /// Speech playback has completed
    /// Sent by speaker actuator when TTS audio finishes playing
    /// Used to synchronize state transitions (Speaking → Ready)
    SpeechComplete,

    /// Ambient noise level detected (0-100)
    /// Could be used to detect if user is in a meeting, music playing, etc.
    AmbientNoiseLevel(u8),

    // ============================================================================
    // System Events
    // ============================================================================
    /// System has completed initialization and is ready for interaction
    /// Sent by audio sensor when Vosk model finishes loading
    /// Triggers transition from loading state (red LEDs) to ready state (green LEDs)
    SystemReady,

    // ============================================================================
    // Camera Sensor Events (Phase 2+)
    // ============================================================================
    /// Camera detected a human face/person
    /// confidence: 0.0-1.0 representing detection confidence
    // TODO: Add this variant with named field
    // HumanDetected { confidence: f32 },

    /// Desk is currently occupied (person present at desk)
    // TODO: Add this variant
    // DeskOccupied,

    /// Something changed in the camera's view
    /// description: AI-generated description of what changed
    // TODO: Add this variant with named field
    // ObjectChange { description: String },

    // ============================================================================
    // Environmental Sensor Events (Phase 2+)
    // ============================================================================
    /// Temperature and humidity reading from DHT11 sensor
    // TODO: Add this variant with named fields for temp (f32) and humidity (f32)
    // EnvironmentReading { temp: f32, humidity: f32 },

    // ============================================================================
    // User Action Events
    // ============================================================================
    /// User requested Do Not Disturb (Silent) mode
    /// Bot will enter Silent state, showing red breathing LEDs
    UserRequestedDND,

    /// User requested to exit Do Not Disturb mode
    /// Bot will return to Ready state
    UserRequestedWakeUp,

    // ============================================================================
    // System Events
    // ============================================================================
    /// Health status of a component changed
    /// component: name of the component ("pir", "camera", "audio", etc.)
    /// healthy: true if component is working, false if failed
    ComponentHealth { component: String, healthy: bool },
}

impl Event {
    pub fn is_pir_event(&self) -> bool {
        matches!(self, Event::PresenceDetected | Event::NoPresenceSince(_))
    }

    pub fn is_audio_event(&self) -> bool {
        matches!(
            self,
            Event::WakeWordDetected
                | Event::SpeechCaptureStarted
                | Event::SpeechCaptured(_)
                | Event::AmbientNoiseLevel(_)
        )
    }

    // pub fn is_camera_event(&self) -> bool {
    //     matches!(
    //         self,
    //         Event::HumanDetected { .. } | Event::DeskOccupied | Event::ObjectChange { .. }
    //     )
    // }

    // pub fn is_environmental_event(&self) -> bool {
    //     matches!(self, Event::EnvironmentReading { .. })
    // }

    pub fn is_system_event(&self) -> bool {
        matches!(self, Event::ComponentHealth { .. })
    }

    pub fn is_user_action(&self) -> bool {
        matches!(self, Event::UserRequestedDND | Event::UserRequestedWakeUp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let pir_event = Event::PresenceDetected;
        assert!(pir_event.is_pir_event());

        let audio_event = Event::WakeWordDetected;
        assert!(audio_event.is_audio_event());

        let system_event = Event::ComponentHealth {
            component: "camera".to_string(),
            healthy: true,
        };
        assert!(system_event.is_system_event());
    }
}
