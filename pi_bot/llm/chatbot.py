"""LLM generation for the bot."""

from __future__ import annotations

import logging
from collections.abc import Generator
from enum import StrEnum

from ollama import Client
from pydantic import BaseModel

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
        self.max_history = max_history
        self.system_prompt = system_prompt

        self._messages: list[Message] = []

    @property
    def label(self) -> str:
        """Get a human-readable label for the chatbot."""
        return self.__class__.__name__

    @property
    def system_message(self) -> Message:
        """Return the system message for the chatbot."""
        return Message.system_message(content=self.system_prompt)

    @property
    def messages(self) -> list[Message]:
        """Return the list of messages in the conversation."""
        return [self.system_message, *self._messages]

    @property
    def llm_options(self) -> dict:
        """Return the LLM options for the chatbot."""
        return {
            "temperature": self.temperature,
            "num_ctx": self.max_context_length,
            "num_predict": self.num_predict,
        }

    def _add_message(self, message: Message) -> None:
        """Add a message to the conversation.

        :param Message message: The message to add.
        """
        logger.info("[%s] Adding %s message of length %d...", self.label, message.role, len(message.content))
        self._messages.append(message)

        while len(self._messages) > self.max_history:
            self._remove_message(0)

    def _remove_message(self, index: int) -> None:
        """Remove a message from the conversation by index.

        :param int index: The index of the message to remove.
        """
        self._messages.pop(index)

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
                messages=[message.model_dump() for message in self.messages] + [user_message.model_dump()],
                stream=True,
                options=self.llm_options,
            )

            content = ""
            for chunk in stream:
                if chunk_content := chunk.message.content:
                    content += chunk_content
                    yield chunk_content

            assistant_message = Message.assistant_message(content=content)
        except Exception:
            error_msg = f"[{self.label}] Error during chat generation!"
            logger.exception(error_msg)
            raise
        else:
            self._add_message(user_message)
            self._add_message(assistant_message)


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
                print(chunk, end="", flush=True)
    except KeyboardInterrupt:
        logger.info("Chatbot debug stopped by user.")
