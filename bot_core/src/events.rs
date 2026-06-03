//! Events emitted by sensors and system components
//!
//! Events flow from sensors → controller. They represent "something happened"
//! in the physical world or system.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    // ============================================================================
    // Presence & Motion Events
    // ============================================================================
    /// PIR sensor detected motion/presence in the room
    PresenceDetected,

    /// No presence has been detected for the given duration
    /// Used to trigger presence timeout behaviors
    NoPresenceSince(Duration),

    // ============================================================================
    // Audio Events
    // ============================================================================
    /// Wake word ("Hey Bot") was detected by Vosk
    /// This triggers transition from Ready → Active state
    WakeWordDetected,

    /// Speech was captured and transcribed by Vosk
    /// The String contains the transcribed text from the user
    SpeechCaptured(String),

    /// Ambient noise level detected (0-100)
    /// Could be used to detect if user is in a meeting, music playing, etc.
    AmbientNoiseLevel(u8),

    // ============================================================================
    // RFID Events (Phase 1)
    // ============================================================================
    /// RFID tag detected by RC522 reader
    /// tag_id: Unique identifier of the RFID tag
    RfidTagDetected { tag_id: String },

    /// RFID tag is authorized (valid for lock/unlock)
    RfidAuthorized,

    /// RFID tag is not authorized
    RfidUnauthorized,

    // ============================================================================
    // Vision Events (Phase 2+)
    // ============================================================================
    // Hint: These will come from camera_controller in future phases
    // For now, you can leave these commented out or add them as placeholders
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
    // Environmental Events (Phase 2+)
    // ============================================================================
    // Hint: These come from DHT11 sensor

    /// Temperature and humidity reading from DHT11 sensor
    // TODO: Add this variant with named fields for temp (f32) and humidity (f32)
    // EnvironmentReading { temp: f32, humidity: f32 },

    // ============================================================================
    // System Events
    // ============================================================================
    // Hint: These come from health monitoring and error detection

    /// Health status of a component changed
    /// component: name of the component ("pir", "camera", "audio", etc.)
    /// healthy: true if component is working, false if failed
    ComponentHealth { component: String, healthy: bool },
}

impl Event {
    pub fn is_presence_event(&self) -> bool {
        matches!(self, Event::PresenceDetected | Event::NoPresenceSince(_))
    }

    pub fn is_audio_event(&self) -> bool {
        matches!(self, Event::WakeWordDetected | Event::SpeechCaptured(_))
    }

    pub fn is_rfid_event(&self) -> bool {
        matches!(
            self,
            Event::RfidTagDetected { .. } | Event::RfidAuthorized | Event::RfidUnauthorized
        )
    }

    // pub fn is_vision_event(&self) -> bool {
    //     matches!(
    //         self,
    //         Event::HumanDetected { .. } | Event::DeskOccupied | Event::ObjectChange { .. }
    //     )
    // }

    // pub fn is_environment_event(&self) -> bool {
    //     matches!(
    //         self,
    //         Event::EnvironmentReading { .. } | Event::ProximityChanged { .. }
    //     )
    // }

    pub fn is_system_event(&self) -> bool {
        matches!(self, Event::ComponentHealth { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let presence_event = Event::PresenceDetected;
        assert!(presence_event.is_presence_event());
        let audio_event = Event::WakeWordDetected;
        assert!(audio_event.is_audio_event());
        let system_event = Event::ComponentHealth {
            component: "camera".to_string(),
            healthy: true,
        };
        assert!(system_event.is_system_event());
    }
}
