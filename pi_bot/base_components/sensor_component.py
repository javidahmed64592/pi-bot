"""Base component for sensor controllers."""

import asyncio
import logging
from abc import ABC, abstractmethod

from pi_bot.models import BotConfig
from pi_bot.protocol import ComponentType, Event

logger = logging.getLogger(__name__)


class SensorComponent(ABC):
    """Base class for sensor components."""

    def __init__(self, config: BotConfig, event_queue: asyncio.Queue) -> None:
        """Initialize the sensor component."""
        self.config = config
        self.event_queue = event_queue
        self._running = False

    @property
    def label(self) -> str:
        """Get a human-readable label for the sensor component."""
        return self.__class__.__name__

    @property
    @abstractmethod
    def component_type(self) -> ComponentType:
        """Get the component type this sensor handles."""
        pass

    async def emit_event(self, event: Event) -> None:
        """Emit an event to the shared event queue.

        :param Event event: The event to emit.
        """
        await self.event_queue.put(event)
        logger.debug("[%s] Emitted event: %s", self.label, event.event_type)

    @abstractmethod
    async def run(self) -> None:
        """Run the sensor's monitoring loop."""
        self._running = True
        logger.info("[%s] Starting event monitoring loop...", self.label)

    def stop(self) -> None:
        """Signal the sensor to stop monitoring."""
        logger.info("[%s] Stop signal received.", self.label)
        self._running = False
