"""Main module for the Pi Bot application."""

import argparse
import asyncio
import logging

from template_python.logging_setup import setup_default_logging

from pi_bot.actuator.buzzer import debug as buzzer_debug
from pi_bot.actuator.lcd import debug as lcd_debug
from pi_bot.actuator.led import debug as led_debug
from pi_bot.actuator.rgb_led import debug as rgb_led_debug
from pi_bot.config import load_config
from pi_bot.sensor.pir import debug as pir_debug

setup_default_logging()
logger = logging.getLogger(__name__)


def main() -> None:
    """Main function to run the Pi Bot."""
    error_msg = "The main function is not implemented yet."
    logger.error(error_msg)
    raise NotImplementedError(error_msg)


def debug() -> None:
    """Debug function to test loading the configuration."""
    parser = argparse.ArgumentParser(description="Debug the Pi Bot's components.")
    parser.add_argument(
        "component",
        choices=["pir", "rgb_led", "led", "buzzer", "lcd"],
        help="The component to test",
    )
    args = parser.parse_args()

    config = load_config()

    match args.component:
        case "pir":
            pir_debug(pir_pin=config.gpio.pir_pin)
        case "rgb_led":
            asyncio.run(rgb_led_debug(config=config))
        case "led":
            asyncio.run(led_debug(config=config))
        case "buzzer":
            buzzer_debug(buzzer_pin=config.gpio.buzzer_pin, buzzer_tunes_config=config.buzzer_tunes)
        case "lcd":
            lcd_debug(lcd_gpio_config=config.gpio.lcd, lcd_config=config.lcd)
