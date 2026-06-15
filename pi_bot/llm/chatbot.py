"""LLM generation for the bot."""

from __future__ import annotations

import logging
import re
from collections.abc import Generator
from enum import StrEnum
from pathlib import Path

from ollama import Client
from pydantic import BaseModel

from pi_bot.config import DATA_DIRECTORY
from pi_bot.models import BotConfig

logger = logging.getLogger(__name__)


class RoleType(StrEnum):
    """Role types for the chatbot."""

    SYSTEM = "system"
    ASSISTANT = "assistant"
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
    filepath: Path = DATA_DIRECTORY / "chat_history.json"

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
        self.tmp_filepath.write_text(self.model_dump_json(), encoding="utf-8")
        self.tmp_filepath.replace(self.filepath)

    def load(self) -> None:
        """Load the chat history from a JSON file."""
        if self.filepath.exists():
            logger.info("[MessageList] Loading chat history from: %s", self.filepath)
            loaded = MessageList.model_validate_json(self.filepath.read_text(encoding="utf-8"))

            if len(loaded_messages := loaded.messages) == 0:
                logger.info("[MessageList] No messages found in chat history.")

            self.messages = loaded_messages
        else:
            logger.info("[MessageList] No chat history found at: %s", self.filepath)


class Chatbot:
    """A chatbot that uses the Ollama API to generate responses."""

    def __init__(
        self,
        ollama_host: str,
        model_name: str,
        temperature: float,
        max_context_length: int,
        num_predict: int,
        max_history: int,
        system_prompt: str,
    ) -> None:
        """Initialize the chatbot with the given parameters.

        :param str ollama_host: The host URL for the Ollama API.
        :param str model_name: The name of the model to use for generation.
        :param float temperature: The temperature for generation.
        :param int max_context_length: The maximum context length for the model.
        :param int num_predict: The number of predictions to generate.
        :param int max_history: The maximum number of messages to keep in history.
        :param str system_prompt: The system prompt to use for the chatbot.
        """
        if "localhost" in ollama_host:
            logger.info("[%s] Using LOCAL Ollama host.", self.label)
        else:
            logger.info("[%s] Using REMOTE Ollama host.", self.label)

        self.client = Client(host=ollama_host)
        logger.info("[%s] Initialized Ollama client.", self.label)

        self.model_name = model_name
        self.temperature = temperature
        self.max_context_length = max_context_length
        self.num_predict = num_predict

        self.messages: MessageList = MessageList(
            system_message=Message.system_message(content=system_prompt), max_history=max_history
        )
        self.messages.load()

    @property
    def label(self) -> str:
        """Get a human-readable label for the chatbot."""
        return self.__class__.__name__

    @property
    def llm_options(self) -> dict:
        """Return the LLM options for the chatbot."""
        return {
            "temperature": self.temperature,
            "num_ctx": self.max_context_length,
            "num_predict": self.num_predict,
        }

    @staticmethod
    def _iter_sentences(buffer: str, chunk: str) -> Generator[tuple[str, str]]:
        """Append chunk to buffer and yield (sentence, updated_buffer) for each complete sentence.

        :param str buffer: The current incomplete sentence buffer.
        :param str chunk: The latest token chunk from the LLM.
        :return: Yields (sentence, remaining_buffer) tuples for each complete sentence found.
        :rtype: Generator[tuple[str, str], None, None]
        """
        buffer += chunk
        parts = re.split(r"(?<=[.!?])\s+", buffer)
        for sentence in parts[:-1]:
            if sentence := sentence.strip():
                yield sentence, parts[-1]
        yield "", parts[-1]

    def chat(self, user_input: str) -> Generator[str]:
        """Generate a response from the chatbot given user input.

        :param str user_input: The user input to send to the chatbot.
        :return: A generator yielding chunks of the chatbot's response.
        :rtype: Generator[str]
        """
        logger.info("[%s] Sending message to chatbot...", self.label)
        try:
            user_message = Message.user_message(content=user_input)
            stream = self.client.chat(
                model=self.model_name,
                messages=[*self.messages.history_dump, user_message.model_dump()],
                stream=True,
                options=self.llm_options,
            )

            content = ""
            buffer = ""

            for chunk in stream:
                if chunk_content := chunk.message.content:
                    content += chunk_content
                    for sentence, remaining in self._iter_sentences(buffer, chunk_content):
                        buffer = remaining
                        if sentence:
                            yield sentence

            if remainder := buffer.strip():
                yield remainder

            assistant_message = Message.assistant_message(content=content)
        except Exception:
            logger.exception("[%s] Error during chat generation!", self.label)
            raise
        else:
            self.messages.add_message(user_message)
            self.messages.add_message(assistant_message)
            self.messages.save()


def debug(config: BotConfig) -> None:
    """Debug the chatbot by printing the system message and a sample user input."""
    chatbot = Chatbot(
        ollama_host=config.llm.ollama_host,
        model_name=config.llm.model_name,
        temperature=config.llm.temperature,
        max_context_length=config.llm.max_context_length,
        num_predict=config.llm.num_predict,
        max_history=config.llm.max_history,
        system_prompt=config.llm.system_prompt,
    )

    try:
        while True:
            message = str(input("User: "))
            for chunk in chatbot.chat(user_input=message):
                print(chunk, end="\n", flush=True)
    except KeyboardInterrupt:
        logger.info("Chatbot debug stopped by user.")
