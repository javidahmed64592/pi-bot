//! RGB LED Controller Module

use anyhow::Result;
use bot_core::RgbColor;
use rppal::gpio::{Gpio, OutputPin};

/// RGB LED controller with independent PWM control for each color channel
pub struct RgbLedController {
    red_pin: OutputPin,
    green_pin: OutputPin,
    blue_pin: OutputPin,
    label: String,
    current_color: RgbColor,
    /// Current brightness level (0.0-1.0)
    current_brightness: f32,
}

impl RgbLedController {
    /// Initialize RGB LED controller with specified GPIO pins and label
    ///
    /// # Arguments
    /// * `red_pin_number` - GPIO pin number for red channel
    /// * `green_pin_number` - GPIO pin number for green channel
    /// * `blue_pin_number` - GPIO pin number for blue channel
    /// * `label` - Label for logging (e.g., "System RGB LED")
    ///
    /// # Returns
    /// New RGB LED controller instance or error if GPIO initialization fails
    pub fn new(
        red_pin_number: u8,
        green_pin_number: u8,
        blue_pin_number: u8,
        label: &str,
    ) -> Result<Self> {
        let gpio = Gpio::new()?;

        // Initialize all three color channels with PWM at 100Hz
        let mut red_pin = gpio.get(red_pin_number)?.into_output();
        let mut green_pin = gpio.get(green_pin_number)?.into_output();
        let mut blue_pin = gpio.get(blue_pin_number)?.into_output();

        red_pin.set_pwm_frequency(100.0, 0.0)?;
        green_pin.set_pwm_frequency(100.0, 0.0)?;
        blue_pin.set_pwm_frequency(100.0, 0.0)?;

        log::info!(
            "[{}] Initialized on GPIO pins - R:{} G:{} B:{}",
            label,
            red_pin_number,
            green_pin_number,
            blue_pin_number
        );

        Ok(Self {
            red_pin,
            green_pin,
            blue_pin,
            label: label.to_string(),
            current_color: RgbColor::OFF,
            current_brightness: 1.0,
        })
    }

    /// Turn RGB LED on with the last set color at current brightness
    pub fn turn_on(&mut self) {
        self.set_color(self.current_color);
    }

    /// Turn RGB LED off (all channels to 0)
    pub fn turn_off(&mut self) {
        self.set_color(RgbColor::OFF);
    }

    /// Set RGB LED color with current brightness applied
    ///
    /// # Arguments
    /// * `color` - RGB color to display (will be scaled by current brightness)
    pub fn set_color(&mut self, color: RgbColor) {
        // Store the base color (without brightness applied)
        self.current_color = color;

        // Apply brightness scaling (color.scale expects 0.0-1.0)
        let scaled_color = color.scale(self.current_brightness);

        // Convert RGB values (0-255) to PWM duty cycle (0.0-1.0)
        let red_duty = scaled_color.r as f64 / 255.0;
        let green_duty = scaled_color.g as f64 / 255.0;
        let blue_duty = scaled_color.b as f64 / 255.0;

        if let Err(e) = self.red_pin.set_pwm_frequency(100.0, red_duty) {
            log::error!("[{}] Failed to set red PWM: {}", self.label, e);
        }
        if let Err(e) = self.green_pin.set_pwm_frequency(100.0, green_duty) {
            log::error!("[{}] Failed to set green PWM: {}", self.label, e);
        }
        if let Err(e) = self.blue_pin.set_pwm_frequency(100.0, blue_duty) {
            log::error!("[{}] Failed to set blue PWM: {}", self.label, e);
        }
    }

    /// Set overall brightness level (0.0-1.0) without changing color
    ///
    /// # Arguments
    /// * `level` - Brightness level (0.0-1.0, clamped automatically)
    pub fn set_brightness(&mut self, level: f32) {
        self.current_brightness = level.clamp(0.0, 1.0);

        // Re-apply current color with new brightness
        self.set_color(self.current_color);
    }

    /// Get current RGB color (base color without brightness applied)
    pub fn get_color(&self) -> RgbColor {
        self.current_color
    }

    /// Get current brightness level (0.0-1.0)
    pub fn get_brightness(&self) -> f32 {
        self.current_brightness
    }
}
