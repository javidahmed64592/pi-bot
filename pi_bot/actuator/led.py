"""LED control script for the Pi Bot."""

import logging
from time import sleep

from gpiozero import PWMLED

from pi_bot.models import LEDPattern, LEDPatternConfig, LEDPinsConfig, StatusLEDStateConfig

logger = logging.getLogger(__name__)


class LEDController:
    """A simple controller for an LED connected to a GPIO pin."""

    def __init__(self, label: str, pin: int) -> None:
        """Initialize the LED controller with the specified GPIO pin.

        :param str label: A label for the LED (for identification purposes).
        :param int pin: The GPIO pin number to which the LED is connected.
        """
        self.label = label
        self.led = PWMLED(pin)
        logger.info("[%s] Initialized LED on GPIO pin: %d", self.label, pin)

    def on(self) -> None:
        """Turn the LED on."""
        self.led.on()

    def off(self) -> None:
        """Turn the LED off."""
        self.led.off()

    # Patterns
    def pulse(self, interval: float) -> None:
        """Simulate a pulsing pattern by smoothly fading the LED in and out.

        :param float interval: The time in seconds for one pulse cycle (on and off).
        """
        self.led.pulse(fade_in_time=interval / 2, fade_out_time=interval / 2, background=True)

    def blink(self, interval: float) -> None:
        """Simulate a blinking pattern by turning the LED on and off at regular intervals.

        :param float interval: The time in seconds for one blink cycle (on and off).
        """
        self.led.blink(on_time=interval / 2, off_time=interval / 2, background=True)

    def apply_pattern(self, pattern_config: LEDPatternConfig) -> None:
        """Apply the specified LED pattern based on the configuration.

        :param LEDPatternConfig pattern_config: The configuration for the LED pattern to apply.
        """
        match pattern_config.pattern:
            case LEDPattern.OFF:
                logger.info("[%s] Applying LED pattern: OFF", self.label)
                self.off()
            case LEDPattern.SOLID:
                logger.info("[%s] Applying LED pattern: SOLID", self.label)
                self.on()
            case LEDPattern.PULSE:
                logger.info("[%s] Applying LED pattern: PULSE", self.label)
                self.pulse(interval=pattern_config.interval)
            case LEDPattern.BLINK:
                logger.info("[%s] Applying LED pattern: BLINK", self.label)
                self.blink(interval=pattern_config.interval)
            case _:
                error_msg = f"Unsupported LED pattern: {pattern_config.pattern}"
                logger.error("[%s] %s", self.label, error_msg)
                raise ValueError(error_msg)


def debug(led_pins_config: LEDPinsConfig, status_led_config: StatusLEDStateConfig) -> None:
    """Debug function to test the status LEDs."""
    on_time = 5.0
    off_time = 2.0

    green_leds = [
        LEDController(label="Green LED 1", pin=led_pins_config.green_1),
        LEDController(label="Green LED 2", pin=led_pins_config.green_2),
    ]
    red_leds = [
        LEDController(label="Red LED 1", pin=led_pins_config.red_1),
        LEDController(label="Red LED 2", pin=led_pins_config.red_2),
    ]

    def apply_state_pattern(
        on_leds: list[LEDController], off_leds: list[LEDController], pattern_config: LEDPatternConfig
    ) -> None:
        for led in on_leds:
            led.apply_pattern(pattern_config=pattern_config)
        for led in off_leds:
            led.apply_pattern(pattern_config=status_led_config.off)

    def turn_off_all() -> None:
        for led in [*green_leds, *red_leds]:
            led.apply_pattern(pattern_config=status_led_config.off)

    # Test all status LED patterns from config
    logger.info("Testing status LED patterns...")
    sleep(off_time)

    logger.info("1/6 - Loading")
    apply_state_pattern(on_leds=red_leds, off_leds=green_leds, pattern_config=status_led_config.loading)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("2/6 - Ready")
    apply_state_pattern(on_leds=green_leds, off_leds=red_leds, pattern_config=status_led_config.ready)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("3/6 - Observing")
    apply_state_pattern(on_leds=green_leds, off_leds=red_leds, pattern_config=status_led_config.observing)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("4/6 - Silent")
    apply_state_pattern(on_leds=red_leds, off_leds=green_leds, pattern_config=status_led_config.silent)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("5/6 - Active")
    apply_state_pattern(on_leds=green_leds, off_leds=red_leds, pattern_config=status_led_config.active)
    sleep(on_time)

    turn_off_all()
    sleep(off_time)

    logger.info("6/6 - Error")
    apply_state_pattern(on_leds=red_leds, off_leds=green_leds, pattern_config=status_led_config.error)
    sleep(on_time)

    turn_off_all()
    logger.info("Status LED pattern test completed!")
