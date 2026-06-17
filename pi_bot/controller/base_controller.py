"""Base controller for managing bot components."""

import asyncio
import logging
from abc import ABC, abstractmethod

from pi_bot.base_components.actuator_component import ActuatorComponent
from pi_bot.base_components.bidirectional_component import BidirectionalComponent
from pi_bot.base_components.sensor_component import SensorComponent
from pi_bot.protocol import Command, ComponentType

logger = logging.getLogger(__name__)


class BaseController(ABC):
    """Base controller for managing bot components."""

    def __init__(self) -> None:
        """Initialize the base controller."""
        self.event_queue = asyncio.Queue()
        self.command_queues: dict[ComponentType, asyncio.Queue] = {}

        self.actuators: list[ActuatorComponent] = []
        self.sensors: list[SensorComponent] = []
        self.bidirectional: list[BidirectionalComponent] = []

        self.tasks: list[asyncio.Task] = []

    def register_actuators(self, actuators: list[ActuatorComponent]) -> None:
        """Register actuator components with the controller.

        :param list[ActuatorComponent] actuators: The actuators to register.
        """
        self.actuators.extend(actuators)
        for actuator in actuators:
            self.command_queues[actuator.component_type] = actuator.command_queue
            logger.info("[BaseController] Registered actuator: %s", actuator.label)

    def register_sensors(self, sensors: list[SensorComponent]) -> None:
        """Register sensor components with the controller.

        :param list[SensorComponent] sensors: The sensors to register.
        """
        self.sensors.extend(sensors)
        for sensor in sensors:
            logger.info("[BaseController] Registered sensor: %s", sensor.label)

    def register_bidirectionals(self, components: list[BidirectionalComponent]) -> None:
        """Register bidirectional components with the controller.

        :param list[BidirectionalComponent] components: The bidirectional components to register.
        """
        self.bidirectional.extend(components)
        for component in components:
            self.command_queues[component.component_type] = component.command_queue
            logger.info("[BaseController] Registered bidirectional component: %s", component.label)

    async def start(self) -> None:
        """Start all registered components in non-blocking mode."""
        logger.info("[BaseController] Starting all components...")

        for actuator in self.actuators:
            task = asyncio.create_task(actuator.run())
            self.tasks.append(task)
            logger.info("[BaseController] Started actuator task: %s", actuator.label)

        for sensor in self.sensors:
            task = asyncio.create_task(sensor.run())
            self.tasks.append(task)
            logger.info("[BaseController] Started sensor task: %s", sensor.label)

        for component in self.bidirectional:
            task = asyncio.create_task(component.run())
            self.tasks.append(task)
            logger.info("[BaseController] Started bidirectional task: %s", component.label)

        logger.info("[BaseController] All components ready!")

    async def stop(self) -> None:
        """Stop all components and wait for graceful shutdown."""
        logger.info("[BaseController] Stopping all components...")

        for actuator in self.actuators:
            actuator.stop()

        for sensor in self.sensors:
            sensor.stop()

        for component in self.bidirectional:
            component.stop()

        for task in self.tasks:
            task.cancel()

        await asyncio.gather(*self.tasks, return_exceptions=True)

        logger.info("[BaseController] All components stopped.")

    async def send_command(self, command: Command) -> None:
        """Send a command to the appropriate actuator's queue.

        :param Command command: The command to send.
        :raises ValueError: If no queue exists for the target component.
        """
        if (queue := self.command_queues.get(command.component)) is None:
            error_msg = f"No queue registered for component: {command.component}"
            logger.error("[BaseController] %s", error_msg)
            raise ValueError(error_msg)

        await queue.put(command)
        logger.debug("[BaseController] Sent command to %s: %s", command.command_type, command.component)

    @abstractmethod
    async def update(self) -> None:
        """Update the controller's state."""
        await asyncio.sleep(0.1)

    async def run(self) -> None:
        """Run the central controller's main loop."""
        try:
            await self.start()

            while True:
                try:
                    await self.update()

                except asyncio.CancelledError:
                    logger.info("[BaseController] Main loop cancelled.")
                    raise
                except Exception:
                    logger.exception("[BaseController] Error in main loop!")

        finally:
            await self.stop()
