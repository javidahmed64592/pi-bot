"""Pydantic models for the Pi Bot."""

from __future__ import annotations

from enum import StrEnum
from pathlib import Path

import yaml
from pydantic import BaseModel, Field


# GPIO Pin Mappings
class RGBPinsConfig(BaseModel):
    """Configuration for RGB LED pin mappings."""

    red: int = Field(..., description="GPIO pin number for the red channel of the RGB LED.", ge=0, le=27)
    green: int = Field(..., description="GPIO pin number for the green channel of the RGB LED.", ge=0, le=27)
    blue: int = Field(..., description="GPIO pin number for the blue channel of the RGB LED.", ge=0, le=27)


class LEDPinsConfig(BaseModel):
    """Configuration for the status LEDs pin mappings."""

    green_1: int = Field(..., description="GPIO pin number for the first green status LED.", ge=0, le=27)
    green_2: int = Field(..., description="GPIO pin number for the second green status LED.", ge=0, le=27)
    red_1: int = Field(..., description="GPIO pin number for the first red status LED.", ge=0, le=27)
    red_2: int = Field(..., description="GPIO pin number for the second red status LED.", ge=0, le=27)


class GPIOConfig(BaseModel):
    """Configuration for GPIO pin mappings."""

    # Sensors (inputs)
    pir_pin: int = Field(..., description="GPIO pin number for the PIR motion sensor.", ge=0, le=27)

    # Actuators (outputs)
    rgb_pins: RGBPinsConfig = Field(..., description="Configuration for RGB LED pin mappings.")
    led_pins: LEDPinsConfig = Field(..., description="Configuration for status LED pin mappings.")
    lcd_i2c_address: int = Field(..., description="I2C address for the LCD display.", ge=0x03, le=0x77)


# Audio Configuration
class VoskSTTConfig(BaseModel):
    """Speech capture settings for Vosk speech recognition."""

    capture_timeout: float = Field(
        ..., description="Maximum duration (in seconds) to capture audio for speech recognition.", ge=0.1, le=60.0
    )
    silence_timeout: float = Field(
        ..., description="Silence duration (in seconds) to end speech capture.", ge=0.1, le=10.0
    )
    min_speech_duration: float = Field(
        ..., description="Minimum speech duration (in seconds) to process.", ge=0.1, le=10.0
    )


class VoskConfig(BaseModel):
    """Configuration for Vosk speech recognition."""

    model_name: str = Field(..., description="Name of the Vosk model to use for speech recognition.")
    wake_phrases: list[str] = Field(..., description="List of wake phrases to activate the bot.")
    keyword_mode: bool = Field(..., description="Whether or not to use keyword mode for Vosk.")
    stt: VoskSTTConfig = Field(..., description="Speech capture settings for Vosk speech recognition.")

    @property
    def model_path(self) -> Path:
        """Get the path to the Vosk model directory."""
        return Path("vosk") / self.model_name


class PiperConfig(BaseModel):
    """Configuration for Piper Text-to-Speech."""

    model_name: str = Field(..., description="Name of the Piper voice to use for text-to-speech.")

    @property
    def model_path(self) -> Path:
        """Get the path to the Piper voice ONNX file."""
        return Path("piper") / f"{self.model_name}.onnx"

    @property
    def config_path(self) -> Path:
        """Get the path to the Piper voice configuration file."""
        return self.model_path.with_suffix(".json")


class AudioConfig(BaseModel):
    """Configuration for audio settings."""

    microphone_device: str = Field(..., description="Name of the microphone device to use for audio input.")
    speaker_device: str = Field(..., description="Name of the speaker device to use for audio output.")
    sample_rate: int = Field(..., description="Audio sample rate in Hz.", ge=8000, le=48000)
    vosk: VoskConfig = Field(..., description="Configuration for Vosk speech recognition.")
    piper: PiperConfig = Field(..., description="Configuration for Piper Text-to-Speech.")


# LLM Configuration
class LLMConfig(BaseModel):
    """Configuration for the language model."""

    model_name: str = Field(..., description="Name of the language model to use for generating responses.")
    ollama_host: str = Field(..., description="URL of the Ollama server hosting the language model.")
    temperature: float = Field(..., description="Sampling temperature for the language model.", ge=0.1, le=2.0)
    max_context_length: int = Field(..., description="Maximum context length for the language model.")
    system_prompt: str = Field(..., description="System prompt to guide the language model's behavior.")


# Memory Configuration
class EmbeddingsConfig(BaseModel):
    """Configuration for text embeddings."""

    model_name: str = Field(..., description="Name of the embedding model to use for vectorizing text.")
    dimensions: int = Field(..., description="Dimensionality of the embedding vectors.", ge=1, le=4096)

    @property
    def model_path(self) -> Path:
        """Get the path to the embedding model ONNX file."""
        return Path("embeddings") / f"{self.model_name}.onnx"

    @property
    def tokenizer_path(self) -> Path:
        """Get the path to the embedding model tokenizer configuration file."""
        return f"{self.model_path.with_suffix('')}_tokenizer.json"


class MemorySearchConfig(BaseModel):
    """Configuration for memory search settings."""

    top_k: int = Field(..., description="Number of top similar memories to retrieve during search.", ge=1, le=100)
    min_similarity: float = Field(
        ..., description="Minimum cosine similarity (0.0-1.0) to consider a memory relevant.", ge=0.0, le=1.0
    )
    max_facts: int = Field(..., description="Maximum number of facts to store in memory.", ge=1, le=10000)


class MemoryConfig(BaseModel):
    """Configuration for the Pi Bot's memory system."""

    session_storage_directory: str = Field(..., description="Directory name to store session data and memory files.")
    long_term_storage_directory: str = Field(..., description="Directory name to store long-term memory files.")
    max_session_length: int = Field(..., description="Maximum number of messages to keep in short-term memory.", ge=1)
    embeddings: EmbeddingsConfig = Field(..., description="Configuration for text embeddings.")
    search: MemorySearchConfig = Field(..., description="Configuration for memory search settings.")


# Bot Behaviour
class Observation(BaseModel):
    """Probability weighting for a specific observation."""

    minutes_per_step: float = Field(
        ..., description="Number of minutes per probability increment step.", ge=1.0, le=60.0
    )
    max_steps: int = Field(..., description="Maximum number of steps for probability increment.", ge=1, le=100)
    bonus_per_step: float = Field(..., description="Bonus probability increment per step.", ge=0.0, le=1.0)


class ObservationProbability(BaseModel):
    """Configuration for observation probability weighting."""

    base: float = Field(
        ..., description="Base probability for collecting passive observations and speaking.", ge=0.0, le=1.0
    )
    ceiling: float = Field(
        ..., description="Maximum probability ceiling for collecting passive observations and speaking.", ge=0.0, le=1.0
    )

    # Observation types
    presence: Observation = Field(..., description="Probability weighting for user presence.")
    interaction: Observation = Field(..., description="Probability weighting for time since last interaction.")


class BotBehaviourConfig(BaseModel):
    """Configuration for the Pi Bot's behaviour and interaction settings."""

    passive_observation_interval: tuple[int, int] = Field(
        ..., description="Random time interval (in seconds) for checking passive observations.", example=(60, 300)
    )
    conversation_timeout: int = Field(
        ..., description="Seconds of silence before considering conversation ended.", ge=1
    )
    idle_timeout: int = Field(..., description="Seconds before going idle if no presence detected.", ge=1)
    do_not_disturb_duration: int = Field(..., description="Seconds to stay in DND mode after user leaves.", ge=1)
    observation_probability: ObservationProbability = Field(
        ..., description="Configuration for observation probability weighting."
    )


# LED Configurations
class LEDPattern(StrEnum):
    """Predefined LED patterns for the Pi Bot."""

    OFF = "off"
    SOLID = "solid"
    PULSE = "pulse"
    BLINK = "blink"
    GRADIENT = "gradient"
    RAINBOW = "rainbow"


class RGBColour(BaseModel):
    """Configuration for RGB LED colours."""

    red: int = Field(..., description="Red component of the RGB colour (0-255).", ge=0, le=255)
    green: int = Field(..., description="Green component of the RGB colour (0-255).", ge=0, le=255)
    blue: int = Field(..., description="Blue component of the RGB colour (0-255).", ge=0, le=255)


class LEDPatternConfig(BaseModel):
    """Configuration for LED patterns."""

    pattern: LEDPattern = Field(..., description="LED pattern to display.")
    interval: float = Field(..., description="Interval in seconds for pattern cycle.", ge=0.0, le=10.0)


class RGBLEDPatternConfig(LEDPatternConfig):
    """Configuration for RGB LED patterns."""

    colours: list[RGBColour] = Field(..., description="RGB colours to use for the LED pattern.")


class RGBLEDActiveStateConfig(BaseModel):
    """Configuration for RGB patterns for active conversation sub-states."""

    listening: RGBLEDPatternConfig = Field(..., description="LED pattern configuration for the 'listening' state.")
    thinking: RGBLEDPatternConfig = Field(..., description="LED pattern configuration for the 'thinking' state.")
    speaking: RGBLEDPatternConfig = Field(..., description="LED pattern configuration for the 'speaking' state.")
    learning: RGBLEDPatternConfig = Field(..., description="LED pattern configuration for the 'learning' state.")


class RGBLEDStateConfig(BaseModel):
    """Configuration for RGB patterns for different bot states."""

    ready: RGBLEDPatternConfig = Field(..., description="LED pattern configuration for the 'ready' state.")
    observing: RGBLEDPatternConfig = Field(..., description="LED pattern configuration for the 'observing' state.")
    silent: RGBLEDPatternConfig = Field(..., description="LED pattern configuration for the 'silent' state.")
    active: RGBLEDActiveStateConfig = Field(
        ..., description="LED pattern configuration for active conversation sub-states."
    )


class StatusLEDStateConfig(BaseModel):
    """Configuration for patterns for the status LEDs."""

    loading: LEDPatternConfig = Field(..., description="LED pattern configuration for the 'loading' state.")
    ready: LEDPatternConfig = Field(..., description="LED pattern configuration for the 'ready' state.")
    observing: LEDPatternConfig = Field(..., description="LED pattern configuration for the 'observing' state.")
    silent: LEDPatternConfig = Field(..., description="LED pattern configuration for the 'silent' state.")
    active: LEDPatternConfig = Field(..., description="LED pattern configuration for the 'active' conversation state.")
    error: LEDPatternConfig = Field(..., description="LED pattern configuration for the 'error' state.")


# Main Bot Configuration
class BotConfig(BaseModel):
    """Main configuration model for the Pi Bot."""

    gpio: GPIOConfig = Field(..., description="Configuration for GPIO pin mappings.")
    audio: AudioConfig = Field(..., description="Configuration for audio settings.")
    llm: LLMConfig = Field(..., description="Configuration for the language model.")
    memory: MemoryConfig = Field(..., description="Configuration for the Pi Bot's memory system.")
    behaviour: BotBehaviourConfig = Field(
        ..., description="Configuration for the Pi Bot's behaviour and interaction settings."
    )
    rgb_led_patterns: RGBLEDStateConfig = Field(
        ..., description="Configuration for RGB LED patterns for different bot states."
    )
    status_led_patterns: StatusLEDStateConfig = Field(
        ..., description="Configuration for patterns for the status LEDs."
    )

    @classmethod
    def from_yaml(cls, filepath: Path) -> BotConfig:
        """Load the bot configuration from a YAML file."""
        with filepath.open() as f:
            data = yaml.safe_load(f)
        return cls(**data)
