"""Event and command protocol definitions for the bot."""

from enum import StrEnum, auto
from pydantic import BaseModel, Field

from pi_bot.models import LEDPatternConfig, RGBLEDPatternConfig, MusicalNoteConfig, LCDMessageConfig


# Types
class ComponentType(StrEnum):
    """Enumeration of component types for the bot."""

    # Actuators
    STATUS_LED = auto()
    RGB_LED = auto()
    BUZZER = auto()
    LCD = auto()

    # Sensors
    PIR = auto()

    # Audio
    MICROPHONE = auto()
    SPEAKER = auto()


class StatusLEDType(StrEnum):
    """Enumeration of status LED types."""

    GREEN = auto()
    RED = auto()


class EventType(StrEnum):
    """Enumeration of event types emitted by sensors."""

    MOTION_DETECTED = auto()
    WAKE_WORD_DETECTED = auto()
    SPEECH_CAPTURED = auto()
    STARTED_SPEAKING = auto()
    STOPPED_SPEAKING = auto()


class CommandType(StrEnum):
    """Enumeration of command types sent to actuators."""

    SET_LED_PATTERN = auto()
    PLAY_TUNE = auto()
    WRITE_LCD_TEXT = auto()
    START_LISTENING = auto()
    STOP_LISTENING = auto()
    SPEAK_TEXT = auto()


# Base classes
class Payload(BaseModel):
    """Base class for event and command payloads."""


class Event(BaseModel):
    """Model for events emitted by sensors."""

    component: ComponentType = Field(..., description="The component that emitted the event.")
    event_type: EventType = Field(..., description="The type of event emitted.")
    payload: Payload = Field(..., description="The payload of the event.")


class Command(BaseModel):
    """Model for commands sent to actuators."""

    component: ComponentType = Field(..., description="The component that the command is for.")
    command_type: CommandType = Field(..., description="The type of command to execute.")
    payload: Payload = Field(..., description="The payload of the command.")


# Payloads
class SpeechCapturedPayload(Payload):
    """Payload for speech captured events."""

    transcribed_text: str = Field(..., description="The transcribed text from the captured speech.")


class SetLEDPatternPayload(Payload):
    """Payload for LED pattern commands."""

    pattern_config: LEDPatternConfig = Field(..., description="The configuration for the LED pattern to apply.")
    on_led_type: StatusLEDType = Field(..., description="The type of LED to turn on (GREEN or RED).")


class SetRGBLEDPatternPayload(Payload):
    """Payload for RGB LED pattern commands."""

    pattern_config: RGBLEDPatternConfig = Field(..., description="The configuration for the RGB LED pattern to apply.")


class PlayTunePayload(Payload):
    """Payload for playing a musical tune."""

    tune: list[MusicalNoteConfig] = Field(..., description="The sequence of musical notes to play.")


class WriteLCDTextPayload(Payload):
    """Payload for writing text to the LCD display."""

    message: LCDMessageConfig = Field(..., description="The message to display on the LCD.")


class SpeakTextPayload(Payload):
    """Payload for speaking text through the speaker."""

    text: str = Field(..., description="The text to speak through the speaker.")
