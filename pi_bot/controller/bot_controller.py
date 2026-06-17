"""Central controller for managing bot components."""

import asyncio
import logging

from pi_bot.actuator import BuzzerActuator, LCDActuator, LEDActuator, RGBLEDActuator
from pi_bot.audio import MicrophoneSensor, SpeakerActuator
from pi_bot.controller.base_controller import BaseController
from pi_bot.models import BotConfig
from pi_bot.sensor import PIRSensor

logger = logging.getLogger(__name__)


class BotController(BaseController):
    """Base controller for managing bot components."""

    def __init__(self, config: BotConfig) -> None:
        """Initialize the bot controller with the given configuration.

        :param BotConfig config: The configuration for the bot.
        """
        super().__init__()
        self.config = config

        # Actuators
        self.buzzer_actuator = BuzzerActuator(config=config)
        self.lcd_actuator = LCDActuator(config=config)
        self.led_actuator = LEDActuator(config=config)
        self.rgb_led_actuator = RGBLEDActuator(config=config)

        self.register_actuators(
            [
                self.buzzer_actuator,
                self.lcd_actuator,
                self.led_actuator,
                self.rgb_led_actuator,
            ]
        )

        # Sensors
        self.pir_sensor = PIRSensor(config=config, event_queue=self.event_queue)

        self.register_sensors(
            [
                self.pir_sensor,
            ]
        )

        # Audio
        self.microphone_sensor = MicrophoneSensor(config=config, event_queue=self.event_queue)
        self.speaker_actuator = SpeakerActuator(config=config, event_queue=self.event_queue)

        self.register_bidirectionals(
            [
                self.microphone_sensor,
                self.speaker_actuator,
            ]
        )

        logger.info("[BotController] BotController initialized!")

    async def update(self) -> None:
        """Update the controller state and process events."""
        await asyncio.sleep(1)
