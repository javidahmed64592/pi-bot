"""LCD1602 display control module for I2C interface."""

import logging
from time import sleep

import smbus2 as smbus

from pi_bot.models import LCDConfig, LCDMessageConfig


logger = logging.getLogger(__name__)


class LCDController:
    """A simple controller for an LCD1602 display via I2C interface."""

    def __init__(self, address: int, bus_number: int, display_time: int) -> None:  # noqa: FBT001, FBT002
        """Initialize the LCD1602 display.

        :param int address: I2C address of the LCD display.
        :param int bus_number: I2C bus number.
        :param int display_time: Seconds to display messages on LCD.
        """
        self.address = address
        self.bus_number = bus_number
        self.display_time = display_time
        self.bus = smbus.SMBus(self.bus_number)
        self.backlight_enabled = False
        self._initialize_display()
        self.set_backlight(self.backlight_enabled)

    def _write_word(self, data: int) -> None:
        """Write a byte to the LCD display.

        :param int data: The byte to write.
        """
        temp = data
        if self.backlight_enabled:
            temp |= 0x08
        else:
            temp &= 0xF7
        self.bus.write_byte(self.address, temp)  # type: ignore[union-attr]

    def _send_command(self, command: int) -> None:
        """Send a command to the LCD display.

        :param int command: The command byte to send.
        """
        # Send bit7-4 firstly
        buf = command & 0xF0
        buf |= 0x04  # RS = 0, RW = 0, EN = 1
        self._write_word(buf)
        sleep(0.002)
        buf &= 0xFB  # Make EN = 0
        self._write_word(buf)

        # Send bit3-0 secondly
        buf = (command & 0x0F) << 4
        buf |= 0x04  # RS = 0, RW = 0, EN = 1
        self._write_word(buf)
        sleep(0.002)
        buf &= 0xFB  # Make EN = 0
        self._write_word(buf)

    def _send_data(self, data: int) -> None:
        """Send data to the LCD display.

        :param int data: The data byte to send.
        """
        # Send bit7-4 firstly
        buf = data & 0xF0
        buf |= 0x05  # RS = 1, RW = 0, EN = 1
        self._write_word(buf)
        sleep(0.002)
        buf &= 0xFB  # Make EN = 0
        self._write_word(buf)

        # Send bit3-0 secondly
        buf = (data & 0x0F) << 4
        buf |= 0x05  # RS = 1, RW = 0, EN = 1
        self._write_word(buf)
        sleep(0.002)
        buf &= 0xFB  # Make EN = 0
        self._write_word(buf)

    def _initialize_display(self) -> None:
        """Initialize the LCD display with proper settings."""
        try:
            self._send_command(0x33)  # Must initialize to 8-line mode at first
            sleep(0.005)
            self._send_command(0x32)  # Then initialize to 4-line mode
            sleep(0.005)
            self._send_command(0x28)  # 2 Lines & 5*7 dots
            sleep(0.005)
            self._send_command(0x0C)  # Enable display without cursor
            sleep(0.005)
            self._send_command(0x01)  # Clear Screen
            self.bus.write_byte(self.address, 0x08)  # type: ignore[union-attr]
            logger.info("LCD1602 display initialized successfully at address 0x%02X", self.address)
        except Exception:
            logger.exception("Failed to initialize LCD1602 display!")
            raise

    def _write(self, x: int, y: int, text: str) -> None:
        """Write text to the LCD display at the specified position.

        :param int x: Column position (0-15).
        :param int y: Row position (0-1).
        :param str text: Text to display.
        """
        # TODO: Add support for text scrolling if text exceeds display width

        # Constrain coordinates to valid ranges
        x = max(0, min(15, x))
        y = max(0, min(1, y))

        # Move cursor to position
        address = 0x80 + 0x40 * y + x
        self._send_command(address)

        # Write each character
        try:
            for char in text:
                self._send_data(ord(char))
        except Exception:
            logger.exception("Error writing text to LCD display!")

    def write(self, message: LCDMessageConfig) -> None:
        """Write a message to the LCD display based on the provided configuration.

        :param LCDMessageConfig message: The message configuration containing text and position.
        """
        self._write(x=message.line_1.column, y=0, text=message.line_1.text)
        self._write(x=message.line_2.column, y=1, text=message.line_2.text)

    def clear(self) -> None:
        """Clear the LCD display."""
        try:
            self._send_command(0x01)
        except Exception:
            logger.exception("Error clearing LCD display!")

    def set_backlight(self, enabled: bool) -> None:  # noqa: FBT001
        """Enable or disable the LCD backlight.

        :param bool enabled: True to enable backlight, False to disable.
        """
        self.backlight_enabled = enabled
        if enabled:
            self.bus.write_byte(self.address, 0x08)  # type: ignore[union-attr]
        else:
            self.bus.write_byte(self.address, 0x00)  # type: ignore[union-attr]

    def _cleanup_component(self) -> None:
        """Clean up I2C bus resources."""
        try:
            self.set_backlight(False)
            if self.bus:
                self.bus.close()
                self.bus = None
            logger.info("LCD1602 cleanup complete.")
        except Exception:
            logger.exception("Error during LCD cleanup!")

    def cleanup(self) -> None:
        """Clean up I2C bus resources."""
        self._cleanup_component()

def get_lcd_controller(config: LCDConfig) -> LCDController:
    """Factory function to create an LCDController instance based on the provided configuration.

    :param LCDConfig config: The configuration for the LCD display, including I2C address and bus number.
    :return: An instance of LCDController.
    :rtype: LCDController
    """
    return LCDController(address=config.i2c_address, bus_number=config.bus_number, display_time=config.display_time)

def debug(config: LCDConfig) -> None:
    """Demonstrate LCD1602 functionality."""
    off_time = 3.0

    lcd = get_lcd_controller(config=config)

    # Test LCD display
    logger.info("Testing LCD display...")
    sleep(off_time)

    logger.info("1/3 - Toggle Display")
    lcd.set_backlight(True)
    sleep(off_time)
    lcd.set_backlight(False)
    sleep(off_time)

    logger.info("2/3 - Display Startup Message")
    lcd.set_backlight(True)
    sleep(off_time)
    lcd.write(message=config.startup_message)
    sleep(lcd.display_time)
    lcd.set_backlight(False)
    sleep(off_time)

    logger.info("3/3 - Clear Display")
    lcd.set_backlight(True)
    sleep(lcd.display_time)
    lcd.clear()
    sleep(off_time)
    lcd.set_backlight(False)

    logger.info("LCD tests complete!")
