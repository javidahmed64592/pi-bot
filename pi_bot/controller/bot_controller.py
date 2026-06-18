"""Central controller for managing bot components."""

import asyncio
import datetime
import logging
from enum import StrEnum, auto

from pi_bot.actuator import BuzzerActuator, LCDActuator, LEDActuator, RGBLEDActuator
from pi_bot.audio import MicrophoneSensor, SpeakerActuator
from pi_bot.controller.base_controller import BaseController
from pi_bot.controller.commands import (
    create_finish_speaking_command,
    create_play_tune_command,
    create_set_led_pattern_command,
    create_set_rgb_led_pattern_command,
    create_speak_text_command,
    create_start_listening_command,
    create_start_transcription_command,
    create_stop_listening_command,
    create_write_lcd_text_command,
)
from pi_bot.llm.chatbot import Chatbot
from pi_bot.models import BotConfig
from pi_bot.protocol import Event, EventType, Payload, SpeechCapturedPayload, StatusLEDType
from pi_bot.sensor import PIRSensor

logger = logging.getLogger(__name__)


class ConversationState(StrEnum):
    """Enumeration of conversation states for the Pi Bot."""

    # Base states
    LOADING = auto()
    READY = auto()
    OBSERVING = auto()
    SILENT = auto()

    # Active states
    LISTENING = auto()
    SPEAKING = auto()

    @property
    def is_active(self) -> bool:
        """Check if the conversation state is active (LISTENING or SPEAKING).

        :return: True if the state is active, False otherwise.
        """
        return self in {ConversationState.LISTENING, ConversationState.SPEAKING}


class BotController(BaseController):
    """Base controller for managing bot components."""

    def __init__(self, config: BotConfig) -> None:
        """Initialize the bot controller with the given configuration.

        :param BotConfig config: The configuration for the bot.
        """
        super().__init__()
        self.config = config
        self.conversation_state: ConversationState
        self.chatbot = Chatbot(
            ollama_host=config.llm.ollama_host,
            model_name=config.llm.model_name,
            temperature=config.llm.temperature,
            max_context_length=config.llm.max_context_length,
            num_predict=config.llm.num_predict,
            max_history=config.llm.max_history,
            system_prompt=config.llm.system_prompt,
            embeddings_model_name=config.llm.embeddings.model_name,
            embeddings_temperature=config.llm.embeddings.temperature,
            top_k=config.llm.embeddings.top_k,
            add_similarity_threshold=config.llm.embeddings.add_similarity_threshold,
            retrieve_similarity_threshold=config.llm.embeddings.retrieve_similarity_threshold,
            max_facts=config.llm.embeddings.max_facts,
            tools=[],
        )

        self._last_user_presence_timestamp = BotController.get_current_timestamp()

    @staticmethod
    def get_current_timestamp() -> float:
        """Get the current timestamp in seconds since the epoch.

        :return: The current timestamp.
        """
        return datetime.datetime.now(datetime.UTC).timestamp()

    async def start(self) -> None:
        """Start fast components first, show loading state, then load slow audio models."""
        logger.info("[BotController] Starting bot...")

        # Actuators
        self.buzzer_actuator = BuzzerActuator(config=self.config)
        self.lcd_actuator = LCDActuator(config=self.config)
        self.led_actuator = LEDActuator(config=self.config)
        self.rgb_led_actuator = RGBLEDActuator(config=self.config)

        self.register_actuators(
            [
                self.buzzer_actuator,
                self.lcd_actuator,
                self.led_actuator,
                self.rgb_led_actuator,
            ]
        )
        self.start_components(components=self.actuators)

        # Sensors
        self.pir_sensor = PIRSensor(config=self.config, event_queue=self.event_queue)
        self.register_sensors(
            [
                self.pir_sensor,
            ]
        )
        self.start_components(components=self.sensors)

        # Loading state while audio components are being initialized
        await asyncio.sleep(0)
        await self._set_conversation_state(state=ConversationState.LOADING)

        # Audio
        logger.info("[BotController] Loading audio components...")
        loop = asyncio.get_running_loop()
        self.microphone_sensor, self.speaker_actuator = await asyncio.gather(
            loop.run_in_executor(None, lambda: MicrophoneSensor(config=self.config, event_queue=self.event_queue)),
            loop.run_in_executor(None, lambda: SpeakerActuator(config=self.config, event_queue=self.event_queue)),
        )
        self.register_bidirectionals([self.microphone_sensor, self.speaker_actuator])
        self.start_components(components=self.bidirectionals)

        # Ready state after all components have been initialized
        await asyncio.sleep(0)
        await self._set_conversation_state(state=ConversationState.READY)
        logger.info("[BotController] All components started!")

    async def _set_conversation_state(self, state: ConversationState) -> None:
        """Set the conversation state and update the LCD and RGB LED accordingly.

        :param ConversationState state: The new conversation state.
        """
        logger.info("[BotController] Setting conversation state to: %s", state)
        self.conversation_state = state
        match self.conversation_state:
            case ConversationState.LOADING:
                # Boot sequence initialized
                # LEDs display loading pattern, LCD shows startup message, buzzer plays startup tune
                await self.send_command(
                    create_set_rgb_led_pattern_command(pattern_config=self.config.rgb_led_patterns.loading)
                )
                await self.send_command(
                    create_set_led_pattern_command(
                        pattern_config=self.config.status_led_patterns.loading, on_led=StatusLEDType.RED
                    )
                )
                await self.send_command(
                    create_write_lcd_text_command(lcd_message_config=self.config.lcd.startup_message)
                )
                await self.send_command(create_play_tune_command(tune=self.config.buzzer_tunes.startup_tune))

            case ConversationState.READY:
                # Boot sequence completed OR decided not to speak after observing
                # OR finished speaking OR exiting silent mode after user returned to desk
                # LEDs display ready pattern, buzzer plays state up tune
                # Microphone starts listening for wake word
                await self.send_command(
                    create_set_rgb_led_pattern_command(pattern_config=self.config.rgb_led_patterns.ready)
                )
                await self.send_command(
                    create_set_led_pattern_command(
                        pattern_config=self.config.status_led_patterns.ready, on_led=StatusLEDType.GREEN
                    )
                )
                await self.send_command(create_play_tune_command(tune=self.config.buzzer_tunes.state_up_tune))
                await self.send_command(create_start_listening_command())

            case ConversationState.OBSERVING:
                # Enter state on random intervals while in ready state
                # LEDs display observing pattern
                # Microphone stops listening
                await self.send_command(
                    create_set_rgb_led_pattern_command(pattern_config=self.config.rgb_led_patterns.observing)
                )
                await self.send_command(
                    create_set_led_pattern_command(
                        pattern_config=self.config.status_led_patterns.observing, on_led=StatusLEDType.GREEN
                    )
                )
                await self.send_command(create_stop_listening_command())

            case ConversationState.SILENT:
                # Enter state when user leaves desk while in ready state
                # LEDs display silent pattern, buzzer plays state down tune
                # Microphone stops listening
                await self.send_command(
                    create_set_rgb_led_pattern_command(pattern_config=self.config.rgb_led_patterns.silent)
                )
                await self.send_command(
                    create_set_led_pattern_command(
                        pattern_config=self.config.status_led_patterns.silent, on_led=StatusLEDType.RED
                    )
                )
                await self.send_command(create_play_tune_command(tune=self.config.buzzer_tunes.state_down_tune))
                await self.send_command(create_stop_listening_command())

            case ConversationState.LISTENING:
                # Enter state when wake word detected while in ready state
                # LEDs display listening pattern, buzzer plays state up tune
                # Microphone starts transcription
                await self.send_command(
                    create_set_rgb_led_pattern_command(pattern_config=self.config.rgb_led_patterns.active.listening)
                )
                await self.send_command(
                    create_set_led_pattern_command(
                        pattern_config=self.config.status_led_patterns.active, on_led=StatusLEDType.GREEN
                    )
                )
                await self.send_command(create_play_tune_command(tune=self.config.buzzer_tunes.state_up_tune))
                await self.send_command(create_start_transcription_command())

            case ConversationState.SPEAKING:
                # Enter state when speaking after finished thinking
                # LEDs display speaking pattern, buzzer plays state up tune
                # Microphone stops listening
                await self.send_command(
                    create_set_rgb_led_pattern_command(pattern_config=self.config.rgb_led_patterns.active.speaking)
                )
                await self.send_command(
                    create_set_led_pattern_command(
                        pattern_config=self.config.status_led_patterns.active, on_led=StatusLEDType.GREEN
                    )
                )
                await self.send_command(create_play_tune_command(tune=self.config.buzzer_tunes.state_up_tune))
                await self.send_command(create_stop_listening_command())

    def _validate_payload_type(self, payload: Payload, expected_type: type[Payload]) -> bool:
        """Validate that the payload is of the expected type.

        :param Payload payload: The payload to validate.
        :param type[Payload] expected_type: The expected type of the payload.
        :return: True if the payload is of the expected type, False otherwise.
        :rtype: bool
        """
        if not isinstance(payload, expected_type):
            logger.error(
                "[BotController] Invalid payload type: expected %s, got %s",
                expected_type.__name__,
                type(payload).__name__,
            )
            return False
        return True

    async def handle_event(self, event: Event) -> None:
        """Handle events.

        :param Event event: The event to handle.
        """
        logger.info("[BotController] Handling event: %s", event.event_type)
        match event.event_type:
            case EventType.MOTION_DETECTED:
                # Return to ready if in silent
                if self.conversation_state == ConversationState.SILENT:
                    await self._set_conversation_state(state=ConversationState.READY)

                # Update the last user presence timestamp when motion is detected
                self._last_user_presence_timestamp = BotController.get_current_timestamp()

            case EventType.WAKE_WORD_DETECTED:
                # Start transcription
                await self._set_conversation_state(state=ConversationState.LISTENING)

            case EventType.SPEECH_CAPTURED:
                # Send speech to chatbot
                await self._set_conversation_state(state=ConversationState.SPEAKING)

                if not isinstance(payload := event.payload, SpeechCapturedPayload):
                    logger.error(
                        "[BotController] Invalid payload type: expected %s, got %s",
                        SpeechCapturedPayload.__name__,
                        type(payload).__name__,
                    )
                    return

                for chunk in self.chatbot.chat(user_input=payload.transcribed_text):
                    await self.send_command(create_speak_text_command(text=chunk))

                await self.send_command(create_finish_speaking_command())

            case EventType.STOPPED_SPEAKING:
                # Go back to ready state
                await self._set_conversation_state(state=ConversationState.READY)

    async def update(self) -> None:
        """Update the controller state and process events."""
        await asyncio.sleep(1)
