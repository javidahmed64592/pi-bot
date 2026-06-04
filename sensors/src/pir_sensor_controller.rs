//! PIR Sensor Controller
//!
//! Provides hardware abstraction for PIR (Passive Infrared) motion sensors.
//! Detects motion via LOW → HIGH transitions and tracks presence timeout.

use anyhow::Result;
use bot_core::Event;
use rppal::gpio::{Gpio, InputPin, Level};
use std::time::{Duration, Instant};

/// PIR sensor controller with motion detection and timeout tracking
pub struct PirSensorController {
    pin: InputPin,
    label: String,
    previous_state: Level,
    last_motion_time: Option<Instant>,
    timeout_duration: Duration,
    timeout_notified: bool,
}

impl PirSensorController {
    /// Initialize PIR sensor controller
    ///
    /// # Arguments
    /// * `pin_number` - GPIO pin number for PIR sensor data line
    /// * `label` - Label for logging (e.g., "PIR Sensor")
    /// * `timeout_duration` - Duration to wait before emitting NoPresenceSince event
    ///
    /// # Returns
    /// Configured PirSensorController instance
    pub fn new(pin_number: u8, label: &str, timeout_duration: Duration) -> Result<Self> {
        let gpio = Gpio::new()?;
        let pin = gpio.get(pin_number)?.into_input();
        let previous_state = pin.read();

        println!("[{}] Initialized on GPIO {}", label, pin_number);
        println!("[{}] Timeout: {:?}", label, timeout_duration);

        let (last_motion_time, timeout_notified) = if previous_state == Level::High {
            println!("[{}] Initial state: MOTION DETECTED", label);
            (Some(Instant::now()), false)
        } else {
            println!("[{}] Initial state: No motion", label);
            (None, false)
        };

        Ok(Self {
            pin,
            label: label.to_string(),
            previous_state,
            last_motion_time,
            timeout_duration,
            timeout_notified,
        })
    }

    /// Check for motion and timeout events
    ///
    /// # Returns
    /// - `Some(Event::PresenceDetected)` when motion is detected (LOW → HIGH)
    /// - `Some(Event::NoPresenceSince(duration))` when timeout expires without motion
    /// - `None` if no events to report
    ///
    /// # Behavior
    /// Call this method in a polling loop. It will emit PresenceDetected on motion
    /// and NoPresenceSince once after the configured timeout expires.
    pub fn check_motion(&mut self) -> Option<Event> {
        let current_state = self.pin.read();

        // Check for state change (motion detected or ended)
        if current_state != self.previous_state {
            self.previous_state = current_state;

            match current_state {
                Level::High => {
                    // Motion detected (LOW → HIGH transition)
                    println!("[{}] Motion DETECTED!", self.label);
                    self.last_motion_time = Some(Instant::now());
                    self.timeout_notified = false;
                    return Some(Event::PresenceDetected);
                }
                Level::Low => {
                    // PIR hardware timeout expired (HIGH → LOW transition)
                    println!("[{}] PIR hardware timeout (pin LOW)", self.label);
                }
            }
        }

        // Check for presence timeout (if we have a last motion time and haven't notified yet)
        if let Some(last_motion) = self.last_motion_time {
            let elapsed = last_motion.elapsed();
            if elapsed >= self.timeout_duration && !self.timeout_notified {
                println!(
                    "[{}] Presence timeout: no motion for {:?}",
                    self.label, elapsed
                );
                self.timeout_notified = true;
                return Some(Event::NoPresenceSince(elapsed));
            }
        }

        None
    }
}
