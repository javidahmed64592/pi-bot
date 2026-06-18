"""Speaker control script for the bot."""

import asyncio
import logging
from pathlib import Path

import pyaudio
from piper import PiperVoice

from pi_bot.base_components.bidirectional_component import BidirectionalComponent
from pi_bot.config import MODELS_DIRECTORY
from pi_bot.models import BotConfig
from pi_bot.protocol import Command, CommandType, ComponentType, Event, EventType, Payload, SpeakTextPayload

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
        logger.info("[%s] Speaking text of length: %d", self.label, len(text))
        for chunk in self.voice.synthesize(text):
            self.stream.write(chunk.audio_int16_bytes)

    def cleanup(self) -> None:
        """Clean up audio resources."""
        self.stream.stop_stream()
        self.stream.close()
        self.pa.terminate()


class SpeakerActuator(BidirectionalComponent):
    """Bidirectional component for controlling the speaker (TTS)."""

    def __init__(self, config: BotConfig, event_queue: asyncio.Queue) -> None:
        """Initialize the speaker actuator with the specified configuration and queues."""
        super().__init__(config=config, event_queue=event_queue)
        self.speaker = SpeakerController(
            label="PiperTTS", model_path=MODELS_DIRECTORY / self.config.audio.piper.model_path
        )

    @property
    def component_type(self) -> ComponentType:
        """Get the component type this actuator handles."""
        return ComponentType.SPEAKER

    async def handle_command(self, command: Command) -> None:
        """Handle commands for the speaker actuator."""
        match command.command_type:
            case CommandType.SPEAK_TEXT:
                payload: SpeakTextPayload = command.payload

                await asyncio.to_thread(self.speaker.speak, payload.text)

            case CommandType.FINISH_SPEAKING:
                await self.emit_event(
                    Event(
                        component=self.component_type,
                        event_type=EventType.STOPPED_SPEAKING,
                        payload=Payload(),
                    )
                )

            case _:
                error_msg = f"Unsupported command type: {command.command_type}"
                logger.error("[%s] %s", self.label, error_msg)

    def stop(self) -> None:
        """Signal the actuator to stop processing commands."""
        super().stop()
        self.speaker.cleanup()


async def debug(config: BotConfig) -> None:
    """Debug function to test the Piper TTS system."""
    logger.info("Initializing components...")
    event_queue = asyncio.Queue()
    speaker_actuator = SpeakerActuator(config=config, event_queue=event_queue)

    # Start the speaker's command processing loop in the background
    logger.info("Testing Piper TTS system...")
    task = asyncio.create_task(speaker_actuator.run())
    await asyncio.sleep(1.0)

    # Send command to speak text
    test_text = "Hello, this is a test of the Piper text-to-speech system."
    logger.info("Sending text: %s", test_text)
    await speaker_actuator.command_queue.put(
        Command(
            component=ComponentType.SPEAKER,
            command_type=CommandType.SPEAK_TEXT,
            payload=SpeakTextPayload(text=test_text),
        )
    )

    # Wait for Piper to stop speaking
    logger.info("Waiting for event: %s", EventType.STOPPED_SPEAKING)
    event: Event = await event_queue.get()
    if event.event_type != EventType.STOPPED_SPEAKING:
        error_msg = f"Unexpected event received: {event.event_type} from {event.component}"
        logger.error(error_msg)
        raise RuntimeError(error_msg)

    logger.info("Event received: %s", event.event_type)
    event_queue.task_done()

    # Wait for all commands to be processed
    await speaker_actuator.command_queue.join()

    # Cleanup and stop the speaker task
    logger.info("Cleaning up...")
    speaker_actuator.stop()
    task.cancel()

    # Wait for the task to finish cancellation
    try:
        await task
    except asyncio.CancelledError:
        pass

    logger.info("Piper TTS system test complete!")
