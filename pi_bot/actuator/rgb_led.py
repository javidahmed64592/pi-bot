"""RGB LED control script for the bot."""

import logging
from time import sleep

from gpiozero import RGBLED

from pi_bot.models import LEDPattern, RGBColour, RGBLEDPatternConfig, RGBLEDStateConfig, RGBPinsConfig

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
    def pulse(self, interval: float, colour: RGBColour) -> None:
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

    def blink(self, interval: float, colour: RGBColour) -> None:
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
                self.pulse(interval=pattern_config.interval, colour=pattern_config.colours[0])
            case LEDPattern.BLINK:
                logger.info("[%s] Applying LED pattern: BLINK", self.label)
                self.blink(interval=pattern_config.interval, colour=pattern_config.colours[0])
            case _:
                error_msg = f"Unsupported LED pattern: {pattern_config.pattern}"
                logger.error("[%s] %s", self.label, error_msg)
                raise ValueError(error_msg)


def get_rgb_led_controller(rgb_pins_config: RGBPinsConfig) -> RGBLEDController:
    """Create and return an RGB LED controller for the main RGB LED.

    :param RGBPinsConfig rgb_pins_config: The configuration containing GPIO pin numbers for the LEDs.
    :return: An RGBLEDController instance for the main RGB LED.
    """
    return RGBLEDController(
        label="RGB LED",
        red_pin=rgb_pins_config.red,
        green_pin=rgb_pins_config.green,
        blue_pin=rgb_pins_config.blue,
    )


def debug(rgb_pins_config: RGBPinsConfig, rgb_led_config: RGBLEDStateConfig) -> None:  # noqa: PLR0915
    """Debug function to test the RGB LED."""
    on_time = 5.0
    off_time = 2.0

    rgb_led = get_rgb_led_controller(rgb_pins_config=rgb_pins_config)

    def turn_off_all() -> None:
        rgb_led.apply_pattern(
            pattern_config=RGBLEDPatternConfig(
                pattern=LEDPattern.OFF, interval=0.0, colours=[RGBColour(red=0, green=0, blue=0)]
            )
        )

    # Test all RGB LED patterns from config
    logger.info("Testing RGB LED patterns...")
    sleep(off_time)

    logger.info("1/9 - Loading")
    rgb_led.apply_pattern(pattern_config=rgb_led_config.loading)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("2/9 - Ready")
    rgb_led.apply_pattern(pattern_config=rgb_led_config.ready)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("3/9 - Observing")
    rgb_led.apply_pattern(pattern_config=rgb_led_config.observing)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("4/9 - Silent")
    rgb_led.apply_pattern(pattern_config=rgb_led_config.silent)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("5/9 - Active (Listening)")
    rgb_led.apply_pattern(pattern_config=rgb_led_config.active.listening)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("6/9 - Active (Thinking)")
    rgb_led.apply_pattern(pattern_config=rgb_led_config.active.thinking)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("7/9 - Active (Speaking)")
    rgb_led.apply_pattern(pattern_config=rgb_led_config.active.speaking)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("8/9 - Active (Learning)")
    rgb_led.apply_pattern(pattern_config=rgb_led_config.active.learning)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("9/9 - Error")
    rgb_led.apply_pattern(pattern_config=rgb_led_config.error)
    sleep(on_time)

    turn_off_all()
    logger.info("RGB LED pattern test completed!")
