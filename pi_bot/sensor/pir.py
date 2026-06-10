"""PIR sensor control script for the bot."""

from time import sleep
import logging
from gpiozero import MotionSensor

logger= logging.getLogger(__name__)

class PIRController:
    """A simple controller for a PIR motion sensor."""

    def __init__(self, label: str, pin: int) -> None:
        """Initialize the PIR motion sensor.

        :param str label: Label for the PIR sensor.
        :param int pin: GPIO pin number where the PIR sensor is connected.
        """
        self.label = label
        self.sensor = MotionSensor(pin)
        self.polling_interval = 0.1  # Time in seconds between sensor checks
        logger.info("[%s] PIRController initialized on GPIO pin %d.", self.label, pin)

    @property
    def motion_detected(self) -> bool:
        """Check if motion is currently detected by the PIR sensor.

        :return: True if motion is detected, False otherwise.
        :rtype: bool
        """
        if motion_detected := self.sensor.motion_detected:
            logger.info("[%s] Motion detected.", self.label)

        return motion_detected

def get_pir_controller(label: str, pin: int) -> PIRController:
    """Factory function to create a PIRController instance.

    :param str label: Label for the PIR sensor.
    :param int pin: GPIO pin number where the PIR sensor is connected.
    :return: An instance of the PIRController class.
    :rtype: PIRController
    """
    return PIRController(label=label, pin=pin)

def debug(pir_pin: int) -> None:
    """Demonstrate PIR sensor functionality."""
    pir = get_pir_controller(label="PIR Sensor", pin=pir_pin)

    # Test PIR sensor
    logger.info("Testing PIR sensor. Press Ctrl+C to stop.")
    try:
        while True:
            if pir.motion_detected:
                logger.info("Motion detected!")
            sleep(pir.polling_interval)
    except KeyboardInterrupt:
        logger.info("PIR sensor debug stopped by user.")
