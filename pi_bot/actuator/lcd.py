"""LCD1602 display control script for the bot."""

import logging
from time import sleep

import smbus2 as smbus

from pi_bot.models import LCDConfig, LCDMessageConfig, LCDGPIOConfig


logger = logging.getLogger(__name__)


class LCDController:
    """A simple controller for an LCD1602 display via I2C interface."""

    def __init__(self, label: str, address: int, bus_number: int) -> None:  # noqa: FBT001, FBT002
        """Initialize the LCD1602 display.

        :param str label: Label for the LCD.
        :param int address: I2C address of the LCD.
        :param int bus_number: I2C bus number.
        """
        self.label = label
        self.address = address
        self.bus_number = bus_number
        self.bus = smbus.SMBus(self.bus_number)
        self.backlight_enabled = False
        self._initialize_display()
        self.set_backlight(self.backlight_enabled)
        logger.info(
            "[%s] LCDController initialized at I2C address 0x%02X on bus %d.", self.label, self.address, self.bus_number
        )

    def _write_word(self, data: int) -> None:
        """Write a byte to the LCD.

        :param int data: The byte to write.
        """
        temp = data
        if self.backlight_enabled:
            temp |= 0x08
        else:
            temp &= 0xF7
        self.bus.write_byte(self.address, temp)  # type: ignore[union-attr]

    def _send_command(self, command: int) -> None:
        """Send a command to the LCD.

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
        """Send data to the LCD.

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
        """Initialize the LCD with proper settings."""
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
            logger.info("[%s] Display initialized successfully.", self.label)
        except Exception:
            logger.exception("[%s] Failed to initialize display!", self.label)
            raise

    def _write(self, x: int, y: int, text: str) -> None:
        """Write text to the LCD at the specified position.

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
            logger.exception("[%s] Error writing text to display!", self.label)

    def write(self, message: LCDMessageConfig) -> None:
        """Write a message to the LCD based on the provided configuration.

        :param LCDMessageConfig message: The message configuration containing text and position.
        """
        logger.info("[%s] Writing message...", self.label)
        self._write(x=message.line_1.column, y=0, text=message.line_1.text)
        self._write(x=message.line_2.column, y=1, text=message.line_2.text)

    def clear(self) -> None:
        """Clear the LCD."""
        try:
            logger.info("[%s] Clearing...", self.label)
            self._send_command(0x01)
        except Exception:
            logger.exception("[%s] Error clearing display!", self.label)

    def set_backlight(self, enabled: bool) -> None:  # noqa: FBT001
        """Enable or disable the LCD backlight.

        :param bool enabled: True to enable backlight, False to disable.
        """
        match enabled:
            case True:
                logger.info("[%s] Enabling backlight...", self.label)
                byte_to_write = 0x08
            case False:
                logger.info("[%s] Disabling backlight...", self.label)
                byte_to_write = 0x00

        try:
            self.bus.write_byte(self.address, byte_to_write)  # type: ignore[union-attr]
            self.backlight_enabled = enabled
        except Exception:
            logger.exception("[%s] Error setting backlight!", self.label)

    def cleanup(self) -> None:
        """Clean up I2C bus resources."""
        try:
            self.set_backlight(False)
            if self.bus:
                self.bus.close()
                self.bus = None
            logger.info("[%s] Cleanup complete.", self.label)
        except Exception:
            logger.exception("[%s] Error during cleanup!", self.label)


def get_lcd_controller(config: LCDGPIOConfig) -> LCDController:
    """Factory function to create an LCDController instance based on the provided configuration.

    :param LCDGPIOConfig config: The configuration for the LCD display, including I2C address and bus number.
    :return: An instance of LCDController.
    :rtype: LCDController
    """
    return LCDController(
        label="LCD", address=config.i2c_address, bus_number=config.bus_number
    )


def debug(lcd_gpio_config: LCDGPIOConfig, lcd_config: LCDConfig) -> None:
    """Demonstrate LCD1602 functionality."""
    off_time = 3.0

    lcd = get_lcd_controller(config=lcd_gpio_config)

    # Test LCD display
    logger.info("Testing LCD display...")
    sleep(off_time)

    logger.info("1/4 - Toggle Display")
    lcd.set_backlight(True)
    sleep(off_time)
    lcd.set_backlight(False)
    sleep(off_time)

    logger.info("2/4 - Display Startup Message")
    lcd.set_backlight(True)
    sleep(off_time)
    lcd.write(message=lcd_config.startup_message)
    sleep(lcd_config.display_time)
    lcd.set_backlight(False)
    sleep(off_time)

    logger.info("3/4 - Clear Display")
    lcd.set_backlight(True)
    sleep(lcd_config.display_time)
    lcd.clear()
    sleep(off_time)
    lcd.set_backlight(False)
    sleep(off_time)

    logger.info("4/4 - Cleanup")
    lcd.cleanup()

    logger.info("LCD tests complete!")
