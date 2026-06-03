//! Status LED Controller Module
//!
//! Generic controller for single-color LEDs with PWM brightness control.
//! Used for status indicators (green/red LEDs) but works with any single-color LED.

use anyhow::Result;
use rppal::gpio::{Gpio, OutputPin};

/// Status LED controller with PWM brightness control
pub struct StatusLedController {
    pin: OutputPin,
    label: String,
    /// Current brightness level (0.0-1.0)
    current_brightness: f32,
    /// Previous non-zero brightness for restore on turn_on (0.0-1.0)
    previous_brightness: f32,
}

impl StatusLedController {
    /// Initialize LED controller with specified GPIO pin and label
    ///
    /// # Arguments
    /// * `pin_number` - GPIO pin number the LED is connected to
    /// * `label` - Label for logging (e.g., "Green LED 1", "Red LED 2")
    ///
    /// # Returns
    /// New LED controller instance or error if GPIO initialization fails
    pub fn new(pin_number: u8, label: &str) -> Result<Self> {
        let gpio = Gpio::new()?;
        let mut pin = gpio.get(pin_number)?.into_output();

        // Set up PWM at 100Hz frequency (starts at 0% duty cycle)
        pin.set_pwm_frequency(100.0, 0.0)?;

        log::info!(
            "[{}] Initialized on GPIO pin {} with PWM support",
            label,
            pin_number
        );
        Ok(Self {
            pin,
            label: label.to_string(),
            current_brightness: 0.0,
            previous_brightness: 1.0, // Default to full brightness when first turned on
        })
    }

    /// Turn LED on (restores previous brightness level)
    pub fn turn_on(&mut self) {
        self.set_brightness(self.previous_brightness);
    }

    /// Turn LED off (preserves previous brightness for later restore)
    pub fn turn_off(&mut self) {
        self.set_brightness(0.0);
    }

    /// Set brightness using PWM (0.0-1.0)
    ///
    /// # Arguments
    /// * `level` - Brightness level (0.0-1.0, clamped automatically)
    pub fn set_brightness(&mut self, level: f32) {
        let level = level.clamp(0.0, 1.0);

        // Remember non-zero brightness for when we turn back on
        if level > 0.0 {
            self.previous_brightness = level;
        }

        self.current_brightness = level;

        // Set PWM duty cycle directly (already 0.0-1.0)
        if let Err(e) = self.pin.set_pwm_frequency(100.0, level as f64) {
            log::error!("[{}] Failed to set PWM: {}", self.label, e);
        }
    }

    /// Get current brightness level (0.0-1.0)
    pub fn get_brightness(&self) -> f32 {
        self.current_brightness
    }
}
