"""LED control script for the bot."""

import asyncio
import logging

from gpiozero import PWMLED

from pi_bot.base_components import ActuatorComponent
from pi_bot.models import BotConfig, LEDPattern, LEDPatternConfig
from pi_bot.protocol import Command, CommandType, ComponentType, SetLEDPatternPayload, StatusLEDType

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
    def _pulse(self, interval: float) -> None:
        """Simulate a pulsing pattern by smoothly fading the LED in and out.

        :param float interval: The time in seconds for one pulse cycle (on and off).
        """
        self.led.pulse(fade_in_time=interval / 2, fade_out_time=interval / 2, background=True)

    def _blink(self, interval: float) -> None:
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
                self._pulse(interval=pattern_config.interval)
            case LEDPattern.BLINK:
                logger.info("[%s] Applying LED pattern: BLINK", self.label)
                self._blink(interval=pattern_config.interval)
            case _:
                error_msg = f"Unsupported LED pattern: {pattern_config.pattern}"
                logger.error("[%s] %s", self.label, error_msg)
                raise ValueError(error_msg)


class LEDActuator(ActuatorComponent):
    """Actuator component for controlling the bot's status LEDs."""

    def __init__(self, config: BotConfig, command_queue: asyncio.Queue) -> None:
        """Initialize the LED actuator with the specified configuration and command queue."""
        super().__init__(config=config, command_queue=command_queue)
        self.green_leds = [
            LEDController(label="Green LED 1", pin=self.config.gpio.led_pins.green_1),
            LEDController(label="Green LED 2", pin=self.config.gpio.led_pins.green_2),
        ]
        self.red_leds = [
            LEDController(label="Red LED 1", pin=self.config.gpio.led_pins.red_1),
            LEDController(label="Red LED 2", pin=self.config.gpio.led_pins.red_2),
        ]

    @property
    def component_type(self) -> ComponentType:
        """Get the component type this actuator handles."""
        return ComponentType.STATUS_LED

    def handle_command(self, command: Command) -> None:
        """Handle commands for the LED actuator."""
        super().handle_command(command)
        match command.command_type:
            case CommandType.SET_LED_PATTERN:
                payload: SetLEDPatternPayload = command.payload
                self._apply_status_led_pattern(on_led_type=payload.on_led_type, pattern_config=payload.pattern_config)
            case _:
                error_msg = f"Unsupported command type: {command.command_type}"
                logger.error("[%s] %s", self.label, error_msg)

    def _apply_status_led_pattern(self, on_led_type: StatusLEDType, pattern_config: LEDPatternConfig) -> None:
        """Apply the specified pattern to the 'on' LEDs and turn off the 'off' LEDs.

        :param StatusLEDType on_led_type: The type of LED to turn on (GREEN or RED).
        :param LEDPatternConfig pattern_config: The configuration for the LED pattern to apply to the 'on' LEDs.
        """
        match on_led_type:
            case StatusLEDType.GREEN:
                on_leds = self.green_leds
                off_leds = self.red_leds
            case StatusLEDType.RED:
                on_leds = self.red_leds
                off_leds = self.green_leds

        for led in on_leds:
            led.apply_pattern(pattern_config=pattern_config)
        for led in off_leds:
            led.apply_pattern(pattern_config=LEDPatternConfig(pattern=LEDPattern.OFF, interval=0.0))

    def stop(self) -> None:
        """Signal the actuator to stop processing commands."""
        super().stop()
        for led in [*self.green_leds, *self.red_leds]:
            led.apply_pattern(pattern_config=LEDPatternConfig(pattern=LEDPattern.OFF, interval=0.0))


async def debug(config: BotConfig) -> None:
    """Debug function to test the status LEDs."""
    on_time = 5.0
    off_time = 2.0

    logger.info("Initializing components...")
    command_queue = asyncio.Queue()
    led_actuator = LEDActuator(config=config, command_queue=command_queue)

    def turn_off_all() -> None:
        for led in [*led_actuator.green_leds, *led_actuator.red_leds]:
            led.apply_pattern(pattern_config=LEDPatternConfig(pattern=LEDPattern.OFF, interval=0.0))

    # Start the actuator's command processing loop in the background
    logger.info("Testing status LED patterns...")
    task = asyncio.create_task(led_actuator.run())
    await asyncio.sleep(off_time)

    logger.info("1/6 - Loading")
    await command_queue.put(
        Command(
            component=ComponentType.STATUS_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetLEDPatternPayload(
                pattern_config=config.status_led_patterns.loading, on_led_type=StatusLEDType.RED
            ),
        )
    )
    await asyncio.sleep(on_time)

    turn_off_all()
    await asyncio.sleep(off_time)

    logger.info("2/6 - Ready")
    await command_queue.put(
        Command(
            component=ComponentType.STATUS_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetLEDPatternPayload(
                pattern_config=config.status_led_patterns.ready, on_led_type=StatusLEDType.GREEN
            ),
        )
    )
    await asyncio.sleep(on_time)

    turn_off_all()
    await asyncio.sleep(off_time)

    logger.info("3/6 - Observing")
    await command_queue.put(
        Command(
            component=ComponentType.STATUS_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetLEDPatternPayload(
                pattern_config=config.status_led_patterns.observing, on_led_type=StatusLEDType.GREEN
            ),
        )
    )
    await asyncio.sleep(on_time)

    turn_off_all()
    await asyncio.sleep(off_time)

    logger.info("4/6 - Silent")
    await command_queue.put(
        Command(
            component=ComponentType.STATUS_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetLEDPatternPayload(
                pattern_config=config.status_led_patterns.silent, on_led_type=StatusLEDType.RED
            ),
        )
    )
    await asyncio.sleep(on_time)

    turn_off_all()
    await asyncio.sleep(off_time)

    logger.info("5/6 - Active")
    await command_queue.put(
        Command(
            component=ComponentType.STATUS_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetLEDPatternPayload(
                pattern_config=config.status_led_patterns.active, on_led_type=StatusLEDType.GREEN
            ),
        )
    )
    await asyncio.sleep(on_time)

    turn_off_all()
    await asyncio.sleep(off_time)

    logger.info("6/6 - Error")
    await command_queue.put(
        Command(
            component=ComponentType.STATUS_LED,
            command_type=CommandType.SET_LED_PATTERN,
            payload=SetLEDPatternPayload(
                pattern_config=config.status_led_patterns.error, on_led_type=StatusLEDType.RED
            ),
        )
    )
    await asyncio.sleep(on_time)

    # Wait for all commands to be processed
    logger.info("Waiting for command queue to finish processing...")
    await command_queue.join()

    # Stop the actuator task
    led_actuator.stop()
    task.cancel()

    # Wait for the task to finish cancellation
    try:
        await task
    except asyncio.CancelledError:
        pass

    logger.info("Status LED tests complete!")
