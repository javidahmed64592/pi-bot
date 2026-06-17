"""Passive buzzer control script for the bot."""

import asyncio
import logging
from time import sleep

from gpiozero import TonalBuzzer

from pi_bot.base_components import ActuatorComponent
from pi_bot.models import BotConfig, MusicalNote, MusicalNoteConfig
from pi_bot.protocol import Command, CommandType, ComponentType, PlayTunePayload

logger = logging.getLogger(__name__)


class BuzzerController:
    """A simple controller for a passive buzzer connected to a GPIO pin."""

    def __init__(self, label: str, pin: int) -> None:
        """Initialize the buzzer controller with the specified GPIO pin.

        :param str label: A label for the buzzer.
        :param int pin: The GPIO pin number where the buzzer is connected.
        """
        self.label = label
        self.buzzer = TonalBuzzer(pin)
        logger.info("[%s] Initialized buzzer on GPIO pin: %d", self.label, pin)

    def play(self, note: str) -> None:
        """Play a specific musical note on the buzzer.

        :param str note: The musical note to play.
        """
        if note == MusicalNote.NONE:
            self.stop()
            return
        self.buzzer.play(note)

    def stop(self) -> None:
        """Stop playing any sound on the buzzer."""
        self.buzzer.stop()

    # Play tunes
    def play_tune(self, tune: list[MusicalNoteConfig]) -> None:
        """Play a sequence of musical notes as a tune.

        :param list[MusicalNoteConfig] tune: A list of MusicalNoteConfig objects representing the tune to play.
        """
        logger.info("[%s] Playing tune with %d notes...", self.label, len(tune))
        for note_config in tune:
            self.play(note=note_config.full_note)
            sleep(note_config.duration)
        self.stop()


class BuzzerActuator(ActuatorComponent):
    """Actuator component for controlling the bot's buzzer."""

    def __init__(self, config: BotConfig) -> None:
        """Initialize the buzzer actuator with the specified configuration and command queue."""
        super().__init__(config=config)
        self.buzzer = BuzzerController(label="Buzzer", pin=config.gpio.buzzer_pin)

    @property
    def component_type(self) -> ComponentType:
        """Get the component type this actuator handles."""
        return ComponentType.BUZZER

    def handle_command(self, command: Command) -> None:
        """Handle commands for the buzzer actuator."""
        super().handle_command(command)
        match command.command_type:
            case CommandType.PLAY_TUNE:
                payload: PlayTunePayload = command.payload
                self.buzzer.play_tune(tune=payload.tune)
            case _:
                error_msg = f"Unsupported command type: {command.command_type}"
                logger.error("[%s] %s", self.label, error_msg)


async def debug(config: BotConfig) -> None:
    """Debug function to test the buzzer."""
    off_time = 3.0

    logger.info("Initializing components...")
    buzzer_actuator = BuzzerActuator(config=config)

    # Start the actuator's command processing loop in the background
    logger.info("Testing buzzer tunes...")
    task = asyncio.create_task(buzzer_actuator.run())
    await asyncio.sleep(off_time)

    logger.info("1/5 - Startup Tune")
    await buzzer_actuator.command_queue.put(
        Command(
            component=ComponentType.BUZZER,
            command_type=CommandType.PLAY_TUNE,
            payload=PlayTunePayload(tune=config.buzzer_tunes.startup_tune),
        )
    )
    await asyncio.sleep(off_time)

    logger.info("2/5 - Shutdown Tune")
    await buzzer_actuator.command_queue.put(
        Command(
            component=ComponentType.BUZZER,
            command_type=CommandType.PLAY_TUNE,
            payload=PlayTunePayload(tune=config.buzzer_tunes.shutdown_tune),
        )
    )
    await asyncio.sleep(off_time)

    logger.info("3/5 - State Up Tune")
    await buzzer_actuator.command_queue.put(
        Command(
            component=ComponentType.BUZZER,
            command_type=CommandType.PLAY_TUNE,
            payload=PlayTunePayload(tune=config.buzzer_tunes.state_up_tune),
        )
    )
    await asyncio.sleep(off_time)

    logger.info("4/5 - State Down Tune")
    await buzzer_actuator.command_queue.put(
        Command(
            component=ComponentType.BUZZER,
            command_type=CommandType.PLAY_TUNE,
            payload=PlayTunePayload(tune=config.buzzer_tunes.state_down_tune),
        )
    )
    await asyncio.sleep(off_time)

    logger.info("5/5 - Error Tune")
    await buzzer_actuator.command_queue.put(
        Command(
            component=ComponentType.BUZZER,
            command_type=CommandType.PLAY_TUNE,
            payload=PlayTunePayload(tune=config.buzzer_tunes.error_tune),
        )
    )
    await asyncio.sleep(off_time)

    # Wait for all commands to be processed
    logger.info("Waiting for command queue to finish processing...")
    await buzzer_actuator.command_queue.join()

    # Stop the actuator task
    buzzer_actuator.stop()
    task.cancel()

    # Wait for the task to finish cancellation
    try:
        await task
    except asyncio.CancelledError:
        pass

    logger.info("Buzzer tests complete!")
