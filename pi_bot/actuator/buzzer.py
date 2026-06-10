"""Passive buzzer control script for the bot."""

import logging
from time import sleep

from gpiozero import TonalBuzzer

from pi_bot.models import BuzzerTunesConfig, MusicalNoteConfig, MusicalNote

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


def get_buzzer_controller(pin: int) -> BuzzerController:
    """Factory function to create a BuzzerController instance.

    :param int pin: The GPIO pin number where the buzzer is connected.
    :return: An instance of BuzzerController.
    :rtype: BuzzerController
    """
    return BuzzerController(label="Buzzer", pin=pin)


def debug(buzzer_pin: int, buzzer_tunes_config: BuzzerTunesConfig) -> None:  # noqa: PLR0915
    """Debug function to test the buzzer."""
    off_time = 3.0

    buzzer = get_buzzer_controller(pin=buzzer_pin)

    # Test each tune in the configuration
    logger.info("Testing buzzer tunes...")
    sleep(off_time)

    logger.info("1/5 - Startup Tune")
    buzzer.play_tune(buzzer_tunes_config.startup_tune)
    sleep(off_time)

    logger.info("2/5 - Shutdown Tune")
    buzzer.play_tune(buzzer_tunes_config.shutdown_tune)
    sleep(off_time)

    logger.info("3/5 - State Up Tune")
    buzzer.play_tune(buzzer_tunes_config.state_up_tune)
    sleep(off_time)

    logger.info("4/5 - State Down Tune")
    buzzer.play_tune(buzzer_tunes_config.state_down_tune)
    sleep(off_time)

    logger.info("5/5 - Error Tune")
    buzzer.play_tune(buzzer_tunes_config.error_tune)

    logger.info("Buzzer tune tests complete!")
