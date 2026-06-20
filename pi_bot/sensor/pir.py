"""PIR sensor control script for the bot."""

import asyncio
import logging

from gpiozero import MotionSensor

from pi_bot.base_components.sensor_component import SensorComponent
from pi_bot.models import BotConfig
from pi_bot.protocol import ComponentType, Event, EventType, Payload

logger = logging.getLogger(__name__)


class PIRController:
    """A simple controller for a PIR motion sensor."""

    def __init__(self, label: str, pin: int) -> None:
        """Initialize the PIR motion sensor.

        :param str label: Label for the PIR sensor.
        :param int pin: GPIO pin number where the PIR sensor is connected.
        """
        self.label = label
        self.sensor = MotionSensor(pin)
        self.polling_interval = 1.0  # Time in seconds between sensor checks
        logger.info("[%s] PIRController initialized on GPIO pin %d.", self.label, pin)

    @property
    def motion_detected(self) -> bool:
        """Check if motion is currently detected by the PIR sensor.

        :return: True if motion is detected, False otherwise.
        :rtype: bool
        """
        return self.sensor.is_active


class PIRSensor(SensorComponent):
    """Sensor component for monitoring PIR motion detection."""

    def __init__(self, config: BotConfig, event_queue: asyncio.Queue) -> None:
        """Initialize the PIR sensor with the specified configuration and event queue."""
        super().__init__(config=config, event_queue=event_queue)
        self.pir = PIRController(label="PIR Sensor", pin=self.config.gpio.pir_pin)
        self._motion_active = False
        self._presence_timeout = config.behaviour.presence_timeout
        self._absence_duration = 0.0
        self._left_desk_emitted = False

    @property
    def component_type(self) -> ComponentType:
        """Get the component type this sensor handles."""
        return ComponentType.PIR

    async def run(self) -> None:
        """Run the PIR sensor's monitoring loop."""
        await super().run()

        try:
            while self._running:
                if self.pir.motion_detected:
                    if not self._motion_active:
                        await self.emit_event(
                            Event(
                                component=self.component_type,
                                event_type=EventType.MOTION_DETECTED,
                                payload=Payload(),
                            )
                        )
                        self._motion_active = True

                    self._absence_duration = 0.0
                    self._left_desk_emitted = False

                else:
                    self._motion_active = False
                    self._absence_duration += self.pir.polling_interval

                    if self._absence_duration >= self._presence_timeout and not self._left_desk_emitted:
                        logger.info(
                            "[%s] No motion detected for %.0f seconds — emitting LEFT_DESK.",
                            self.label,
                            self._absence_duration,
                        )
                        await self.emit_event(
                            Event(
                                component=self.component_type,
                                event_type=EventType.LEFT_DESK,
                                payload=Payload(),
                            )
                        )
                        self._left_desk_emitted = True

                await asyncio.sleep(self.pir.polling_interval)

        except asyncio.CancelledError:
            logger.info("[%s] Motion monitoring loop cancelled!", self.label)
            raise
        except Exception:
            logger.exception("[%s] Error in monitoring loop!", self.label)
        finally:
            self._running = False
            logger.info("[%s] Motion monitoring loop stopped.", self.label)


async def debug(config: BotConfig) -> None:
    """Debug function to test the PIR sensor."""
    test_duration = 30.0

    logger.info("Initializing components...")
    event_queue = asyncio.Queue()
    pir_sensor = PIRSensor(config=config, event_queue=event_queue)

    # Start the sensor's monitoring loop in the background
    logger.info("Testing PIR sensor for %.0f seconds...", test_duration)
    task = asyncio.create_task(pir_sensor.run())

    # Monitor events from the queue
    try:
        start_time = asyncio.get_event_loop().time()
        while asyncio.get_event_loop().time() - start_time < test_duration:
            try:
                event: Event = await asyncio.wait_for(event_queue.get(), timeout=1.0)
                logger.info("Event received: %s from %s", event.event_type, event.component)
                event_queue.task_done()
            except TimeoutError:
                pass
    except KeyboardInterrupt:
        logger.info("PIR sensor debug stopped by user")

    # Stop the sensor task
    logger.info("Stopping sensor...")
    pir_sensor.stop()
    task.cancel()

    # Wait for the task to finish cancellation
    try:
        await task
    except asyncio.CancelledError:
        pass

    logger.info("PIR sensor tests complete!")
