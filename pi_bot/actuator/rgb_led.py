"""RGB LED control script for the bot."""

import asyncio
import logging

from gpiozero import RGBLED

from pi_bot.base_components import ActuatorComponent
from pi_bot.models import BotConfig, LEDPattern, RGBColour, RGBLEDPatternConfig
from pi_bot.protocol import Command, CommandType, ComponentType, SetRGBLEDPatternPayload

logger = logging.getLogger(__name__)


class RGBLEDController:
    """A simple controller for an RGB LED connected to GPIO pins."""

    def __init__(self, label: str, red_pin: int, green_pin: int, blue_pin: int) -> None:
        """Initialize the RGB LED controller with the specified GPIO pins.

        :param str label: A label for the LED (for identification purposes).
        :param int red_pin: The GPIO pin number for the red channel.
        :param int green_pin: The GPIO pin number for the green channel.
        :param int blue_pin: The GPIO pin number for the blue channel.
        """
        self.label = label
        self.led = RGBLED(red=red_pin, green=green_pin, blue=blue_pin)
        logger.info("[%s] Initialized RGB LED on GPIO pins: R=%d, G=%d, B=%d", self.label, red_pin, green_pin, blue_pin)

    def _normalise_colour(self, colour: RGBColour) -> tuple[float, float, float]:
        """Convert an RGBColour to a normalized tuple.

        :param RGBColour colour: The RGB colour to normalize.
        :return: A tuple of normalized RGB values (0.0-1.0).
        :rtype: tuple[float, float, float]
        """
        return (colour.red / 255, colour.green / 255, colour.blue / 255)

    def on(self) -> None:
        """Turn the LED on."""
        self.led.on()

    def off(self) -> None:
        """Turn the LED off."""
        self.led.off()

    def set_colour(self, colour: RGBColour) -> None:
        """Set the LED to a specific colour.

        :param RGBColour colour: The RGB colour to set the LED to.
        """
        self.led.color = self._normalise_colour(colour)

    # Patterns
    def _pulse(self, interval: float, colour: RGBColour) -> None:
        """Simulate a pulsing pattern by smoothly fading the LED in and out.

        :param float interval: The time in seconds for one pulse cycle (on and off).
        :param RGBColour colour: The colour to use for the pulse.
        """
        self.led.pulse(
            fade_in_time=interval / 2,
            fade_out_time=interval / 2,
            on_color=self._normalise_colour(colour),
            off_color=(0, 0, 0),
            background=True,
        )

    def _blink(self, interval: float, colour: RGBColour) -> None:
        """Simulate a blinking pattern by turning the LED on and off at regular intervals.

        :param float interval: The time in seconds for one blink cycle (on and off).
        :param RGBColour colour: The colour to use for the blink.
        """
        self.led.blink(
            on_time=interval / 2,
            off_time=interval / 2,
            on_color=self._normalise_colour(colour),
            off_color=(0, 0, 0),
            background=True,
        )

    def apply_pattern(self, pattern_config: RGBLEDPatternConfig) -> None:
        """Apply the specified LED pattern based on the configuration.

        :param RGBLEDPatternConfig pattern_config: The configuration for the LED pattern to apply.
        """
        match pattern_config.pattern:
            case LEDPattern.OFF:
                logger.info("[%s] Applying LED pattern: OFF", self.label)
                self.off()
            case LEDPattern.SOLID:
                logger.info("[%s] Applying LED pattern: SOLID", self.label)
                self.on()
                self.set_colour(colour=pattern_config.colours[0])
            case LEDPattern.PULSE:
                logger.info("[%s] Applying LED pattern: PULSE", self.label)
                self._pulse(interval=pattern_config.interval, colour=pattern_config.colours[0])
            case LEDPattern.BLINK:
                logger.info("[%s] Applying LED pattern: BLINK", self.label)
                self._blink(interval=pattern_config.interval, colour=pattern_config.colours[0])
            case _:
                error_msg = f"Unsupported LED pattern: {pattern_config.pattern}"
                logger.error("[%s] %s", self.label, error_msg)
                raise ValueError(error_msg)


class RGBLEDActuator(ActuatorComponent):
    """Actuator component for controlling the bot's RGB LED."""

    def __init__(self, config: BotConfig) -> None:
        """Initialize the RGB LED actuator with the specified configuration."""
        super().__init__(config=config)
        self.rgb_led = RGBLEDController(
            label="RGB LED",
            red_pin=self.config.gpio.rgb_pins.red,
            green_pin=self.config.gpio.rgb_pins.green,
            blue_pin=self.config.gpio.rgb_pins.blue,
        )

    @property
    def component_type(self) -> ComponentType:
        """Get the component type this actuator handles."""
        return ComponentType.RGB_LED

    def handle_command(self, command: Command) -> None:
        """Handle commands for the LED actuator."""
        super().handle_command(command)
        match command.command_type:
            case CommandType.SET_LED_PATTERN:
                payload: SetRGBLEDPatternPayload = command.payload
                self.rgb_led.apply_pattern(pattern_config=payload.pattern_config)
            case _:
                error_msg = f"Unsupported command type: {command.command_type}"
                logger.error("[%s] %s", self.label, error_msg)

    def stop(self) -> None:
        """Signal the actuator to stop processing commands."""
        super().stop()
        self.rgb_led.apply_pattern(
            pattern_config=RGBLEDPatternConfig(
                pattern=LEDPattern.OFF, interval=0.0, colours=[RGBColour(red=0, green=0, blue=0)]
            )
        )


async def debug(config: BotConfig) -> None:  # noqa: PLR0915
    """Debug function to test the RGB LED."""
    on_time = 5.0
    off_time = 2.0

    logger.info("Initializing components...")
    rgb_led_actuator = RGBLEDActuator(config=config)

    def turn_off_all() -> None:
        rgb_led_actuator.rgb_led.apply_pattern(
            pattern_config=RGBLEDPatternConfig(
                pattern=LEDPattern.OFF, interval=0.0, colours=[RGBColour(red=0, green=0, blue=0)]
            )
        )

    # Start the actuator's command processing loop in the background
    logger.info("Testing RGB LED patterns...")
    task = asyncio.create_task(rgb_led_actuator.run())
    await asyncio.sleep(off_time)

    logger.info("1/7 - Loading")
    await rgb_led_actuator.command_queue.put(
        Command(
            component=ComponentType.RGB_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetRGBLEDPatternPayload(pattern_config=config.rgb_led_patterns.loading),
        )
    )
    await asyncio.sleep(on_time)

    turn_off_all()
    await asyncio.sleep(off_time)

    logger.info("2/7 - Ready")
    await rgb_led_actuator.command_queue.put(
        Command(
            component=ComponentType.RGB_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetRGBLEDPatternPayload(pattern_config=config.rgb_led_patterns.ready),
        )
    )
    await asyncio.sleep(on_time)

    turn_off_all()
    await asyncio.sleep(off_time)

    logger.info("3/7 - Observing")
    await rgb_led_actuator.command_queue.put(
        Command(
            component=ComponentType.RGB_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetRGBLEDPatternPayload(pattern_config=config.rgb_led_patterns.observing),
        )
    )
    await asyncio.sleep(on_time)

    turn_off_all()
    await asyncio.sleep(off_time)

    logger.info("4/7 - Silent")
    await rgb_led_actuator.command_queue.put(
        Command(
            component=ComponentType.RGB_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetRGBLEDPatternPayload(pattern_config=config.rgb_led_patterns.silent),
        )
    )
    await asyncio.sleep(on_time)

    turn_off_all()
    await asyncio.sleep(off_time)

    logger.info("5/7 - Active (Listening)")
    await rgb_led_actuator.command_queue.put(
        Command(
            component=ComponentType.RGB_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetRGBLEDPatternPayload(pattern_config=config.rgb_led_patterns.active.listening),
        )
    )
    await asyncio.sleep(on_time)

    turn_off_all()
    await asyncio.sleep(off_time)

    logger.info("6/7 - Active (Speaking)")
    await rgb_led_actuator.command_queue.put(
        Command(
            component=ComponentType.RGB_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetRGBLEDPatternPayload(pattern_config=config.rgb_led_patterns.active.speaking),
        )
    )
    await asyncio.sleep(on_time)

    turn_off_all()
    await asyncio.sleep(off_time)

    logger.info("7/7 - Error")
    await rgb_led_actuator.command_queue.put(
        Command(
            component=ComponentType.RGB_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetRGBLEDPatternPayload(pattern_config=config.rgb_led_patterns.error),
        )
    )
    await asyncio.sleep(on_time)

    # Wait for all commands to be processed
    logger.info("Waiting for command queue to finish processing...")
    await rgb_led_actuator.command_queue.join()

    # Stop the actuator task
    rgb_led_actuator.stop()
    task.cancel()

    # Wait for the task to finish cancellation
    try:
        await task
    except asyncio.CancelledError:
        pass

    logger.info("RGB LED tests complete!")
