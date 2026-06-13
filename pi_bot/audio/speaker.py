"""Speaker control script for the bot."""

import logging
from pathlib import Path

import pyaudio
from piper import PiperVoice

from pi_bot.models import BotConfig

logger = logging.getLogger(__name__)


class SpeakerController:
    """Speaker controller class using Piper for speech synthesis."""

    def __init__(self, label: str, model_path: Path) -> None:
        """Initialize the speaker controller with a specified Piper voice model.

        :param str label: Label for the speaker controller.
        :param Path model_path: Path to the Piper voice model directory.
        """
        self.label = label
        self.voice = PiperVoice.load(model_path)
        self.pa = pyaudio.PyAudio()
        self.stream = self.pa.open(
            format=pyaudio.paInt16,
            channels=1,
            rate=self.voice.config.sample_rate,
            output=True,
        )
        logger.info("[%s] SpeakerController initialized with model: %s", self.label, model_path)

    def speak(self, text: str) -> None:
        """Convert text to speech and play it through the audio output."""
        for chunk in self.voice.synthesize(text):
            self.stream.write(chunk.audio_int16_bytes)

    def cleanup(self) -> None:
        """Clean up audio resources."""
        self.stream.stop_stream()
        self.stream.close()
        self.pa.terminate()


def debug(config: BotConfig) -> None:
    """Debug function to test the Piper TTS system."""
    logger.info("Initializing components...")
    speaker = SpeakerController(label="PiperTTS", model_path="models" / config.audio.piper.model_path)

    test_text = "Hello, this is a test of the Piper text-to-speech system."
    logger.info("Speaking: %s", test_text)
    speaker.speak(text=test_text)
    speaker.cleanup()
