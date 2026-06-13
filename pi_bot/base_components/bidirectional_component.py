"""Base component for bidirectional controllers (both send events and receive commands)."""

import asyncio
import logging
from abc import ABC, abstractmethod

from pi_bot.models import BotConfig
from pi_bot.protocol import Command, ComponentType, Event

logger = logging.getLogger(__name__)


class BidirectionalComponent(ABC):
    """Base class for bidirectional components that both receive commands and emit events."""

    def __init__(self, config: BotConfig, command_queue: asyncio.Queue, event_queue: asyncio.Queue) -> None:
        """Initialize the bidirectional component.

        :param BotConfig config: The bot configuration.
        :param asyncio.Queue command_queue: Queue for receiving commands.
        :param asyncio.Queue event_queue: Queue for emitting events.
        """
        self.config = config
        self.command_queue = command_queue
        self.event_queue = event_queue
        self._running = False

    @property
    def label(self) -> str:
        """Get a human-readable label for the component."""
        return self.__class__.__name__

    @property
    @abstractmethod
    def component_type(self) -> ComponentType:
        """Get the component type this component handles."""
        pass

    async def emit_event(self, event: Event) -> None:
        """Emit an event to the shared event queue.

        :param Event event: The event to emit.
        """
        await self.event_queue.put(event)
        logger.debug("[%s] Emitted event: %s", self.label, event.event_type)

    @abstractmethod
    async def handle_command(self, command: Command) -> None:
        """Handle a command for the component.

        :param Command command: The command to handle.
        """
        pass

    async def run(self) -> None:
        """Run the component's command processing loop."""
        self._running = True
        logger.info("[%s] Starting command processing loop...", self.label)

        try:
            while self._running:
                try:
                    command: Command = await self.command_queue.get()
                    logger.debug("[%s] Processing command: %s", self.label, command.command_type)

                    await self.handle_command(command)
                    self.command_queue.task_done()
                except asyncio.CancelledError:
                    logger.info("[%s] Command processing loop cancelled!", self.label)
                    raise
                except Exception:
                    logger.exception("[%s] Error processing command!", self.label)

        except asyncio.CancelledError:
            logger.info("[%s] Shutting down command processing loop...", self.label)
        finally:
            self._running = False
            logger.info("[%s] Command processing loop stopped.", self.label)

    def stop(self) -> None:
        """Signal the component to stop processing commands."""
        logger.info("[%s] Stop signal received.", self.label)
        self._running = False
