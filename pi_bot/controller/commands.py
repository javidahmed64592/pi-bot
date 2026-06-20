"""Commands for the bot controller."""

from pi_bot.models import (
    LCDMessageConfig,
    LEDPatternConfig,
    MusicalNoteConfig,
    RGBLEDPatternConfig,
)
from pi_bot.protocol import (
    Command,
    CommandType,
    ComponentType,
    Payload,
    PlayTunePayload,
    SetLEDPatternPayload,
    SetRGBLEDPatternPayload,
    SpeakTextPayload,
    StatusLEDType,
    WriteLCDTextPayload,
)


def create_play_tune_command(tune: list[MusicalNoteConfig]) -> Command:
    """Create a command to play a tune on the buzzer.

    :param list[MusicalNoteConfig] tune: A list of MusicalNoteConfig objects representing the tune to play.
    :return: A Command object to play the specified tune.
    :rtype: Command
    """
    return Command(
        component=ComponentType.BUZZER,
        command_type=CommandType.PLAY_TUNE,
        payload=PlayTunePayload(tune=tune),
    )


def create_write_lcd_text_command(lcd_message_config: LCDMessageConfig) -> Command:
    """Create a command to write text to the LCD display.

    :param LCDMessageConfig lcd_message_config: The LCD message configuration.
    :return: A Command object to write the specified text to the LCD.
    :rtype: Command
    """
    return Command(
        component=ComponentType.LCD,
        command_type=CommandType.WRITE_LCD_TEXT,
        payload=WriteLCDTextPayload(message=lcd_message_config),
    )


def create_set_led_pattern_command(pattern_config: LEDPatternConfig, on_led: StatusLEDType) -> Command:
    """Create a command to set the LED pattern.

    :param LEDPatternConfig pattern_config: The LED pattern configuration.
    :param StatusLEDType on_led: The LED type to apply the pattern to.
    :return: A Command object to set the specified LED pattern.
    :rtype: Command
    """
    return Command(
        component=ComponentType.STATUS_LED,
        command_type=CommandType.SET_LED_PATTERN,
        payload=SetLEDPatternPayload(pattern_config=pattern_config, on_led_type=on_led),
    )


def create_set_rgb_led_pattern_command(pattern_config: RGBLEDPatternConfig) -> Command:
    """Create a command to set the LED pattern.

    :param RGBLEDPatternConfig pattern_config: The RGB LED pattern configuration.
    :return: A Command object to set the specified LED pattern.
    :rtype: Command
    """
    return Command(
        component=ComponentType.RGB_LED,
        command_type=CommandType.SET_LED_PATTERN,
        payload=SetRGBLEDPatternPayload(pattern_config=pattern_config),
    )


def create_start_listening_command() -> Command:
    """Create a command to start listening for user input.

    :return: A Command object to start listening.
    :rtype: Command
    """
    return Command(
        component=ComponentType.MICROPHONE,
        command_type=CommandType.START_LISTENING,
        payload=Payload(),
    )


def create_start_transcription_command() -> Command:
    """Create a command to start transcribing audio input.

    :return: A Command object to start transcription.
    :rtype: Command
    """
    return Command(
        component=ComponentType.MICROPHONE,
        command_type=CommandType.START_TRANSCRIPTION,
        payload=Payload(),
    )


def create_stop_listening_command() -> Command:
    """Create a command to stop listening for user input.

    :return: A Command object to stop listening.
    :rtype: Command
    """
    return Command(
        component=ComponentType.MICROPHONE,
        command_type=CommandType.STOP_LISTENING,
        payload=Payload(),
    )


def create_speak_text_command(text: str) -> Command:
    """Create a command to speak text.

    :param str text: The text to be spoken.
    :return: A Command object to speak the specified text.
    :rtype: Command
    """
    return Command(
        component=ComponentType.SPEAKER,
        command_type=CommandType.SPEAK_TEXT,
        payload=SpeakTextPayload(text=text),
    )


def create_finish_speaking_command() -> Command:
    """Create a command to signal that the bot has finished speaking.

    :return: A Command object to signal the end of speaking.
    :rtype: Command
    """
    return Command(
        component=ComponentType.SPEAKER,
        command_type=CommandType.FINISH_SPEAKING,
        payload=SpeakTextPayload(text=""),
    )
