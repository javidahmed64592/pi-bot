"""Main module for the Pi Bot application."""

import argparse
import logging

from template_python.logging_setup import setup_default_logging

from pi_bot.actuator.led import debug as led_debug
from pi_bot.config import load_config

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
        choices=["led"],
        help="The component to test",
    )
    args = parser.parse_args()

    config = load_config()

    match args.component:
        case "led":
            led_debug(led_pins_config=config.gpio.led_pins, status_led_config=config.status_led_patterns)
        case _:
            error_msg = f"Unsupported component for debugging: {args.component}"
            logger.error(error_msg)
            raise ValueError(error_msg)
