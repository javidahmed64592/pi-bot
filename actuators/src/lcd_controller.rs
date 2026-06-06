//! Generic LCD Controller Module
//!
//! Controller for I2C LCD displays (16x2 with PCF8574 I2C backpack).
//! Uses HD44780 protocol in 4-bit mode with optimized timing.

use anyhow::Result;
use rppal::i2c::I2c;
use std::thread::sleep;
use std::time::Duration;

/// Generic LCD controller for 16x2 I2C displays
pub struct LcdController {
    i2c: I2c,
    label: String,
    backlight_on: bool,
}

// LCD Commands
const LCD_CLEAR: u8 = 0x01;
const LCD_ENTRY_MODE: u8 = 0x04;
const LCD_DISPLAY_CONTROL: u8 = 0x08;
const LCD_FUNCTION_SET: u8 = 0x20;
const LCD_SET_DDRAM_ADDR: u8 = 0x80;

// Flags for display control
const LCD_DISPLAY_ON: u8 = 0x04;
const LCD_CURSOR_OFF: u8 = 0x00;
const LCD_BLINK_OFF: u8 = 0x00;

// Flags for function set
const LCD_4BIT_MODE: u8 = 0x00;
const LCD_2_LINE: u8 = 0x08;
const LCD_5X8_DOTS: u8 = 0x00;

// Flags for backlight control
const LCD_BACKLIGHT: u8 = 0x08;
const LCD_NO_BACKLIGHT: u8 = 0x00;

// Flags for enable bit
const ENABLE: u8 = 0b00000100;

// Timing constants (in microseconds)
const E_PULSE: u64 = 500; // Enable pulse width
const E_DELAY: u64 = 500; // Delay after enable pulse

impl LcdController {
    /// Initialize LCD controller with specified I2C address and label
    ///
    /// # Arguments
    /// * `address` - I2C address of the LCD (typically 0x27 or 0x3F)
    /// * `label` - Label for logging (e.g., "LCD Display", "Main LCD")
    pub fn new(address: u8, label: &str) -> Result<Self> {
        let mut i2c = I2c::new()?;
        i2c.set_slave_address(address as u16)?;

        log::info!("[{}] Initializing on I2C address: 0x{:02X}", label, address);

        let mut lcd = Self {
            i2c,
            label: label.to_string(),
            backlight_on: true,
        };

        // Initialize LCD in 4-bit mode
        lcd.init_lcd()?;

        log::info!("[{}] Initialized successfully", label);
        Ok(lcd)
    }

    /// Initialize the LCD display
    fn init_lcd(&mut self) -> Result<()> {
        sleep(Duration::from_millis(50));

        // Initialize in 4-bit mode
        self.write_four_bits(0x03)?;
        sleep(Duration::from_millis(5));
        self.write_four_bits(0x03)?;
        sleep(Duration::from_millis(5));
        self.write_four_bits(0x03)?;
        sleep(Duration::from_micros(150));
        self.write_four_bits(0x02)?;

        // Function set: 4-bit, 2 line, 5x8 dots
        self.write_command(LCD_FUNCTION_SET | LCD_4BIT_MODE | LCD_2_LINE | LCD_5X8_DOTS)?;

        // Display control: display on, cursor off, blink off
        self.write_command(LCD_DISPLAY_CONTROL | LCD_DISPLAY_ON | LCD_CURSOR_OFF | LCD_BLINK_OFF)?;

        // Clear display
        self.clear()?;

        // Entry mode: increment cursor, no shift
        self.write_command(LCD_ENTRY_MODE | 0x02)?;

        Ok(())
    }

    /// Write a command to the LCD
    fn write_command(&mut self, cmd: u8) -> Result<()> {
        self.write_byte(cmd, false)
    }

    /// Write data to the LCD
    fn write_data(&mut self, data: u8) -> Result<()> {
        self.write_byte(data, true)
    }

    /// Write a byte to the LCD in 4-bit mode
    fn write_byte(&mut self, data: u8, mode: bool) -> Result<()> {
        let high_bits = data & 0xF0;
        let low_bits = (data << 4) & 0xF0;

        self.write_four_bits_with_mode(high_bits, mode)?;
        self.write_four_bits_with_mode(low_bits, mode)?;

        Ok(())
    }

    /// Write four bits with mode flag (command/data)
    fn write_four_bits_with_mode(&mut self, data: u8, mode: bool) -> Result<()> {
        let mode_bit = if mode { 0x01 } else { 0x00 };
        let backlight = if self.backlight_on {
            LCD_BACKLIGHT
        } else {
            LCD_NO_BACKLIGHT
        };
        let byte = data | mode_bit | backlight;

        self.i2c.write(&[byte])?;
        self.strobe_enable(byte)?;

        Ok(())
    }

    /// Write four bits (used during initialization)
    fn write_four_bits(&mut self, data: u8) -> Result<()> {
        let backlight = if self.backlight_on {
            LCD_BACKLIGHT
        } else {
            LCD_NO_BACKLIGHT
        };
        let byte = data | backlight;

        self.i2c.write(&[byte])?;
        self.strobe_enable(byte)?;

        Ok(())
    }

    /// Strobe the enable pin
    fn strobe_enable(&mut self, data: u8) -> Result<()> {
        self.i2c.write(&[data | ENABLE])?;
        sleep(Duration::from_micros(E_PULSE));
        self.i2c.write(&[data & !ENABLE])?;
        sleep(Duration::from_micros(E_DELAY));
        Ok(())
    }

    /// Clear the display
    pub fn clear(&mut self) -> Result<()> {
        self.write_command(LCD_CLEAR)?;
        sleep(Duration::from_millis(2));
        log::info!("[{}] Display cleared", self.label);
        Ok(())
    }

    /// Set cursor position
    pub fn set_cursor(&mut self, line: u8, column: u8) -> Result<()> {
        let row_offsets = [0x00, 0x40];
        if line < 2 {
            let addr = column + row_offsets[line as usize];
            self.write_command(LCD_SET_DDRAM_ADDR | addr)?;
        }
        Ok(())
    }

    /// Write text to a specific line (0 or 1)
    pub fn write_line(&mut self, line: u8, text: &str) -> Result<()> {
        if line >= 2 {
            return Ok(());
        }

        self.set_cursor(line, 0)?;

        // Pad or truncate to 16 characters
        let mut display_text = text.chars().take(16).collect::<String>();
        while display_text.len() < 16 {
            display_text.push(' ');
        }

        for ch in display_text.chars() {
            self.write_data(ch as u8)?;
            // Small delay between characters for stability
            sleep(Duration::from_micros(100));
        }

        log::info!("[{}] Line {}: {}", self.label, line, text);
        Ok(())
    }

    /// Turn backlight on
    pub fn backlight_on(&mut self) -> Result<()> {
        self.backlight_on = true;
        // Re-send last command to update backlight
        self.write_command(LCD_DISPLAY_CONTROL | LCD_DISPLAY_ON | LCD_CURSOR_OFF | LCD_BLINK_OFF)?;
        log::info!("[{}] Backlight ON", self.label);
        Ok(())
    }

    /// Turn backlight off
    pub fn backlight_off(&mut self) -> Result<()> {
        self.backlight_on = false;
        // Re-send last command to update backlight
        self.write_command(LCD_DISPLAY_CONTROL | LCD_DISPLAY_ON | LCD_CURSOR_OFF | LCD_BLINK_OFF)?;
        log::info!("[{}] Backlight OFF", self.label);
        Ok(())
    }
}
