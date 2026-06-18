"""Main module for the Pi Bot application."""

import argparse
import asyncio
import logging
from enum import StrEnum, auto

from template_python.logging_setup import setup_default_logging

from pi_bot.actuator.buzzer import debug as buzzer_debug
from pi_bot.actuator.lcd import debug as lcd_debug
from pi_bot.actuator.led import debug as led_debug
from pi_bot.actuator.rgb_led import debug as rgb_led_debug
from pi_bot.audio.microphone import debug as microphone_debug
from pi_bot.audio.speaker import debug as speaker_debug
from pi_bot.config import load_config
from pi_bot.controller.bot_controller import BotController
from pi_bot.llm.chatbot import debug as chatbot_debug
from pi_bot.sensor.pir import debug as pir_debug

setup_default_logging()
logger = logging.getLogger(__name__)


class DebugOptions(StrEnum):
    """Enumeration of debug options for the Pi Bot."""

    PIR = auto()
    BUZZER = auto()
    LCD = auto()
    LED = auto()
    RGB_LED = auto()
    MICROPHONE = auto()
    SPEAKER = auto()
    CHATBOT = auto()


def main() -> None:
    """Main function to run the Pi Bot."""
    logger.info("Loading configuration...")
    config = load_config()

    logger.info("Initializing BotController...")
    bot_controller = BotController(config=config)

    try:
        logger.info("Starting the controller...")
        asyncio.run(bot_controller.run())
    except KeyboardInterrupt:
        logger.info("Shutting down...")


def debug() -> None:
    """Debug function to test loading the configuration."""
    parser = argparse.ArgumentParser(description="Debug the Pi Bot's components.")
    parser.add_argument(
        "component",
        choices=DebugOptions,
        help="The component to test",
    )
    args = parser.parse_args()

    config = load_config()

    match args.component:
        case DebugOptions.PIR:
            asyncio.run(pir_debug(config=config))
        case DebugOptions.BUZZER:
            asyncio.run(buzzer_debug(config=config))
        case DebugOptions.LCD:
            asyncio.run(lcd_debug(config=config))
        case DebugOptions.LED:
            asyncio.run(led_debug(config=config))
        case DebugOptions.RGB_LED:
            asyncio.run(rgb_led_debug(config=config))
        case DebugOptions.MICROPHONE:
            asyncio.run(microphone_debug(config=config))
        case DebugOptions.SPEAKER:
            asyncio.run(speaker_debug(config=config))
        case DebugOptions.CHATBOT:
            chatbot_debug(config=config)
