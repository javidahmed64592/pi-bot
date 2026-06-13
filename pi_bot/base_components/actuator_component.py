"""Base component for actuator controllers."""

import asyncio
import logging
from abc import ABC, abstractmethod

from pi_bot.models import BotConfig
from pi_bot.protocol import Command, ComponentType

logger = logging.getLogger(__name__)


class ActuatorComponent(ABC):
    """Base class for actuator components."""

    def __init__(self, config: BotConfig, command_queue: asyncio.Queue) -> None:
        """Initialize the actuator component.

        :param BotConfig config: The bot configuration.
        :param asyncio.Queue command_queue: The command queue for this actuator.
        """
        self.config = config
        self.command_queue = command_queue
        self._running = False

    @property
    def label(self) -> str:
        """Get a human-readable label for the actuator component."""
        return self.__class__.__name__

    @property
    @abstractmethod
    def component_type(self) -> ComponentType:
        """Get the component type this actuator handles."""
        pass

    @abstractmethod
    def handle_command(self, command: Command) -> None:
        """Handle a command for the actuator."""
        logger.debug("[%s] Handling command: %s", self.label, command.command_type)

    async def run(self) -> None:
        """Run the actuator's command processing loop."""
        self._running = True
        logger.info("[%s] Starting command processing loop...", self.label)

        try:
            while self._running:
                try:
                    command: Command = await self.command_queue.get()
                    logger.debug("[%s] Processing command: %s", self.label, command.command_type)

                    self.handle_command(command)
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
        """Signal the actuator to stop processing commands."""
        logger.info("[%s] Stop signal received.", self.label)
        self._running = False
