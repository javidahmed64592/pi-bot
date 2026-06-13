"""Base components for hardware integration using the bot protocol."""

from .actuator_component import ActuatorComponent
from .sensor_component import SensorComponent

__all__ = ["ActuatorComponent", "SensorComponent"]
