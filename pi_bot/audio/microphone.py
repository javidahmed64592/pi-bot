"""Microphone control script for the bot."""

import asyncio
import json
import logging
from enum import StrEnum, auto
from pathlib import Path

import pyaudio
from vosk import KaldiRecognizer, Model

from pi_bot.base_components.bidirectional_component import BidirectionalComponent
from pi_bot.config import MODELS_DIRECTORY
from pi_bot.models import BotConfig
from pi_bot.protocol import Command, CommandType, ComponentType, Event, EventType, Payload, SpeechCapturedPayload

logger = logging.getLogger(__name__)


class MicrophoneMode(StrEnum):
    """Enum for microphone modes."""

    IDLE = auto()
    WAKE_WORD = auto()
    TRANSCRIPTION = auto()


class MicrophoneController:
    """Controls the microphone and speech recognition."""

    def __init__(self, label: str, model_path: Path, sample_rate: int, chunk_size: int, wake_words: list[str]) -> None:
        """Initializes the MicrophoneController with the given parameters.

        :param label: Label for the microphone controller.
        :param model_path: Path to the Vosk model directory.
        :param sample_rate: Audio sample rate in Hz.
        :param chunk_size: Size of audio chunks to read from the microphone.
        :param wake_words: List of wake words to detect.
        """
        self.label = label
        self.model = Model(str(model_path))
        self.chunk_size = chunk_size
        self.wake_words = wake_words

        self.wake_recognizer = KaldiRecognizer(self.model, sample_rate)
        self.transcribe_recognizer = KaldiRecognizer(self.model, sample_rate)

        self.pa = pyaudio.PyAudio()
        self.stream = self.pa.open(
            format=pyaudio.paInt16,
            channels=1,
            rate=sample_rate,
            input=True,
            frames_per_buffer=self.chunk_size,
        )
        logger.info("[%s] MicrophoneController initialized with model: %s", self.label, model_path)

    def _read_chunk(self) -> bytes:
        """Reads a chunk of audio data from the microphone."""
        return self.stream.read(self.chunk_size, exception_on_overflow=False)

    def pause_stream(self) -> None:
        """Stop reading from the microphone."""
        self.stream.stop_stream()

    def resume_stream(self, mode: MicrophoneMode) -> None:
        """Resume reading from the microphone."""
        self.stream.start_stream()
        if mode == MicrophoneMode.WAKE_WORD:
            self.wake_recognizer.Reset()
        elif mode == MicrophoneMode.TRANSCRIPTION:
            self.transcribe_recognizer.Reset()

    def detect_wake_word(self) -> bool:
        """Returns True if a wake word is found in the recognised utterance."""
        if self.wake_recognizer.AcceptWaveform(self._read_chunk()):
            text = json.loads(self.wake_recognizer.Result()).get("text", "").lower()
            self.wake_recognizer.Reset()
            return any(word in text for word in self.wake_words)
        return False

    def transcribe(self) -> str | None:
        """Returns transcribed text when an utterance ends, None if still speaking."""
        if self.transcribe_recognizer.AcceptWaveform(self._read_chunk()):
            text = json.loads(self.transcribe_recognizer.Result()).get("text", "").strip()
            self.transcribe_recognizer.Reset()
            return text or None
        return None

    def cleanup(self) -> None:
        """Clean up audio resources."""
        self.stream.stop_stream()
        self.stream.close()
        self.pa.terminate()


class MicrophoneSensor(BidirectionalComponent):
    """Bidirectional component for controlling the microphone (wake word + STT)."""

    def __init__(self, config: BotConfig, event_queue: asyncio.Queue) -> None:
        """Initialize the microphone sensor with the specified configuration and queues."""
        super().__init__(config=config, event_queue=event_queue)
        self.mic = MicrophoneController(
            label="VoskSTT",
            model_path=MODELS_DIRECTORY / self.config.audio.vosk.model_path,
            sample_rate=self.config.audio.vosk.sample_rate,
            chunk_size=self.config.audio.vosk.chunk_size,
            wake_words=self.config.audio.vosk.wake_words,
        )
        self._mode = MicrophoneMode.WAKE_WORD

    @property
    def component_type(self) -> ComponentType:
        """Get the component type this sensor handles."""
        return ComponentType.MICROPHONE

    def extra_tasks(self) -> list:
        """Return the audio monitoring loop as an extra background task."""
        return [self._audio_loop()]

    async def handle_command(self, command: Command) -> None:
        """Handle commands for the microphone sensor."""
        match command.command_type:
            case CommandType.START_LISTENING:
                logger.info("[%s] Resuming stream for wake word detection...", self.label)
                self.mic.resume_stream(mode=MicrophoneMode.WAKE_WORD)
                self._mode = MicrophoneMode.WAKE_WORD
            case CommandType.START_TRANSCRIPTION:
                logger.info("[%s] Resuming stream for transcription...", self.label)
                self.mic.resume_stream(mode=MicrophoneMode.TRANSCRIPTION)
                self._mode = MicrophoneMode.TRANSCRIPTION
            case CommandType.STOP_LISTENING:
                logger.info("[%s] Pausing stream for wake word detection...", self.label)
                self.mic.pause_stream()
                self._mode = MicrophoneMode.IDLE
            case _:
                logger.error("[%s] Unsupported command type: %s", self.label, command.command_type)

    async def _audio_loop(self) -> None:
        """Monitor audio and emit events based on the current mode."""
        while self._running:
            match self._mode:
                case MicrophoneMode.WAKE_WORD:
                    if await asyncio.to_thread(self.mic.detect_wake_word):
                        logger.info("[%s] Wake word detected!", self.label)
                        await self.emit_event(
                            Event(
                                component=self.component_type,
                                event_type=EventType.WAKE_WORD_DETECTED,
                                payload=Payload(),
                            )
                        )

                case MicrophoneMode.TRANSCRIPTION:
                    if text := await asyncio.to_thread(self.mic.transcribe):
                        logger.info("[%s] Speech captured of length: %d", self.label, len(text))
                        await self.emit_event(
                            Event(
                                component=self.component_type,
                                event_type=EventType.SPEECH_CAPTURED,
                                payload=SpeechCapturedPayload(transcribed_text=text),
                            )
                        )

                case MicrophoneMode.IDLE:
                    await asyncio.sleep(0.05)

    def stop(self) -> None:
        """Signal the sensor to stop processing commands and audio."""
        super().stop()
        self.mic.cleanup()


async def debug(config: BotConfig) -> None:
    """Debug function to test the microphone (wake word detection + transcription)."""
    logger.info("Initializing components...")
    event_queue = asyncio.Queue()
    mic_sensor = MicrophoneSensor(config=config, event_queue=event_queue)

    # Start the microphone monitoring loop in the background
    logger.info("Testing Vosk STT system...")
    task = asyncio.create_task(mic_sensor.run())
    await asyncio.sleep(1.0)

    try:
        while True:
            # Step 1: Wait for wake word
            logger.info("Listening for wake words: %s", mic_sensor.mic.wake_words)
            event: Event = await event_queue.get()
            event_queue.task_done()
            logger.info("Event received: %s", event.event_type)

            if event.event_type != EventType.WAKE_WORD_DETECTED:
                continue

            # Step 2: Send command to start transcription
            logger.info("Sending command: %s", CommandType.START_TRANSCRIPTION)
            await mic_sensor.command_queue.put(
                Command(
                    component=ComponentType.MICROPHONE,
                    command_type=CommandType.START_TRANSCRIPTION,
                    payload=Payload(),
                )
            )

            # Step 3: Wait for speech to be captured
            logger.info("Wake word detected - transcribing, speak now...")
            event: Event = await event_queue.get()
            event_queue.task_done()
            logger.info("Event received: %s", event.event_type)

            if event.event_type != EventType.SPEECH_CAPTURED:
                continue

            payload: SpeechCapturedPayload = event.payload
            logger.info("Captured text: '%s'", payload.transcribed_text)

            # Step 4: Send command to stop listening
            logger.info("Sending command: %s", CommandType.STOP_LISTENING)
            await mic_sensor.command_queue.put(
                Command(
                    component=ComponentType.MICROPHONE,
                    command_type=CommandType.STOP_LISTENING,
                    payload=Payload(),
                )
            )

            # Step 5: Simulate controller processing
            logger.info("Simulating controller processing (3s)...")
            await asyncio.sleep(3.0)

            # Step 6: Resume wake word detection
            logger.info("Sending command: %s", CommandType.START_LISTENING)
            await mic_sensor.command_queue.put(
                Command(
                    component=ComponentType.MICROPHONE,
                    command_type=CommandType.START_LISTENING,
                    payload=Payload(),
                )
            )

    except (KeyboardInterrupt, asyncio.CancelledError):
        logger.info("Microphone debug stopped by user.")

    # Cleanup and stop the microphone task
    logger.info("Cleaning up...")
    mic_sensor.stop()
    task.cancel()

    # Wait for the task to finish cancellation
    try:
        await task
    except asyncio.CancelledError:
        pass

    logger.info("Vosk STT system test complete!")
