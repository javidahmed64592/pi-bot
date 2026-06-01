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

// TODO: Phase 1.5 - Implement PirSensorController
// TODO: Phase 1.9 - Implement AudioController for microphone capture

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true);
    }
}
