"""Memory service for the chatbot."""

from __future__ import annotations

import json
import logging
from enum import StrEnum
from pathlib import Path

from pydantic import BaseModel

from pi_bot.config import DATA_DIRECTORY

logger = logging.getLogger(__name__)

SHORT_TERM_MEMORY_FILE = DATA_DIRECTORY / "chat_history.json"
LONG_TERM_MEMORY_FILE = DATA_DIRECTORY / "embeddings.json"


class RoleType(StrEnum):
    """Role types for the chatbot."""

    SYSTEM = "system"
    ASSISTANT = "assistant"
    TOOL = "tool"
    USER = "user"


class Message(BaseModel):
    """A message in the chatbot conversation."""

    role: RoleType
    content: str

    @classmethod
    def system_message(cls, content: str) -> Message:
        """Create a system message.

        :param str content: The content of the system message.
        :return: A system message.
        :rtype: Message
        """
        return cls(role=RoleType.SYSTEM, content=content)

    @classmethod
    def assistant_message(cls, content: str) -> Message:
        """Create an assistant message.

        :param str content: The content of the assistant message.
        :return: An assistant message.
        :rtype: Message
        """
        return cls(role=RoleType.ASSISTANT, content=content)

    @classmethod
    def user_message(cls, content: str) -> Message:
        """Create a user message.

        :param str content: The content of the user message.
        :return: A user message.
        :rtype: Message
        """
        return cls(role=RoleType.USER, content=content)


class MessageList(BaseModel):
    """A list of messages in the chatbot conversation."""

    messages: list[Message] = []
    system_message: Message
    max_history: int
    filepath: Path = SHORT_TERM_MEMORY_FILE

    @property
    def history(self) -> list[Message]:
        """Return the list of messages."""
        return [self.system_message, *self.messages]

    @property
    def history_dump(self) -> list[dict]:
        """Return the list of messages as dictionaries."""
        return [message.model_dump() for message in self.history]

    @property
    def tmp_filepath(self) -> Path:
        """Return the temporary file path for the chat history."""
        return self.filepath.with_suffix(".tmp")

    def add_message(self, message: Message) -> None:
        """Add a message to the list.

        :param Message message: The message to add.
        """
        logger.info("[MessageList] Adding %s message of length %d...", message.role, len(message.content))
        self.messages.append(message)

        while len(self.messages) > self.max_history:
            self.remove_message(0)

    def remove_message(self, index: int) -> None:
        """Remove a message from the list by index.

        :param int index: The index of the message to remove.
        """
        self.messages.pop(index)

    def save(self) -> None:
        """Save the chat history to a JSON file."""
        logger.info("[MessageList] Saving chat history to: %s", self.filepath)
        self.filepath.parent.mkdir(parents=True, exist_ok=True)
        self.tmp_filepath.write_text(
            self.model_dump_json(exclude={"system_message", "max_history", "filepath"}), encoding="utf-8"
        )
        self.tmp_filepath.replace(self.filepath)

    def load(self) -> None:
        """Load the chat history from a JSON file."""
        if self.filepath.exists():
            logger.info("[MessageList] Loading chat history from: %s", self.filepath)
            messages = json.loads(self.filepath.read_text(encoding="utf-8")).get("messages", [])

            if len(loaded_messages := messages) == 0:
                logger.info("[MessageList] No messages found in chat history.")

            self.messages = [Message.model_validate(message) for message in loaded_messages]
            logger.info("[MessageList] Loaded %d messages from chat history.", len(self.messages))
        else:
            logger.info("[MessageList] No chat history found at: %s", self.filepath)
