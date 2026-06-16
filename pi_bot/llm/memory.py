"""Memory service for the chatbot."""

from __future__ import annotations

import datetime
import json
import logging
from collections.abc import Sequence
from enum import StrEnum
from pathlib import Path

import numpy as np
from pydantic import BaseModel

from pi_bot.config import DATA_DIRECTORY

logger = logging.getLogger(__name__)

SHORT_TERM_MEMORY_FILE = DATA_DIRECTORY / "chat_history.json"
LONG_TERM_MEMORY_FILE = DATA_DIRECTORY / "embeddings.json"

EXTRACTOR_INSTRUCTIONS = (
    "You are a memory extraction assistant. Identify concrete, specific facts about the user "
    "from their messages — likes, dislikes, habits, personal details, ongoing projects, people "
    "or things they mention. Be willing to extract simple preference statements like 'user likes X'."
)
EXTRACTION_PROMPT = (
    "Extract specific, personal facts about the user from the following conversation exchange. "
    "Only extract clear, factual statements about the user — not assumptions, opinions, or general topics discussed. "
    "Do not extract facts solely from the assistant's response if the user input does not provide additional context. "
    "Examples of facts worth extracting: preferences, habits, life details, things they own, people they mention, "
    "plans they have.\n\n"
)


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
    """Short-term memory store backed by a JSON file."""

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

    def save(self) -> None:
        """Atomically save the chat history to disk."""
        logger.info("[MessageList] Saving chat history to: %s", self.filepath)
        self.filepath.parent.mkdir(parents=True, exist_ok=True)
        self.tmp_filepath.write_text(
            self.model_dump_json(exclude={"system_message", "max_history", "filepath"}), encoding="utf-8"
        )
        self.tmp_filepath.replace(self.filepath)

    def load(self) -> None:
        """Load the chat history from a JSON file."""
        if not self.filepath.exists():
            logger.info("[MessageList] No chat history found at: %s", self.filepath)
            return

        logger.info("[MessageList] Loading chat history from: %s", self.filepath)
        messages = json.loads(self.filepath.read_text(encoding="utf-8")).get("messages", [])

        if len(loaded_messages := messages) == 0:
            logger.info("[MessageList] No messages found in chat history.")

        self.messages = [Message.model_validate(message) for message in loaded_messages]
        logger.info("[MessageList] Loaded %d messages from chat history.", len(self.messages))

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


class Fact(BaseModel):
    """A stored fact with its embedding vector."""

    text: str
    embedding: Sequence[float]
    created_at: str


class ExtractedFacts(BaseModel):
    """A list of extracted facts."""

    facts: list[str] = []


class MemoryStore(BaseModel):
    """Long-term memory store backed by a JSON file."""

    facts: list[Fact] = []
    filepath: Path = LONG_TERM_MEMORY_FILE

    @property
    def tmp_filepath(self) -> Path:
        """Return the temporary file path for the memory store."""
        return self.filepath.with_suffix(".tmp")

    @staticmethod
    def _cosine_similarity(query: Sequence[float], facts: list[Sequence[float]]) -> np.ndarray:
        """Compute cosine similarity between a query vector and a batch of fact vectors.

        :param Sequence[float] query: The query vector.
        :param list[Sequence[float]] facts: The list of fact vectors.
        :return: An array of cosine similarity scores.
        :rtype: np.ndarray
        """
        query_vec = np.array(query)
        fact_matrix = np.array(facts)

        query_norm = np.linalg.norm(query_vec)
        fact_norms = np.linalg.norm(fact_matrix, axis=1)

        dots = fact_matrix @ query_vec
        denom = fact_norms * query_norm
        denom[denom == 0] = 1e-10

        return dots / denom

    def save(self) -> None:
        """Atomically save the memory store to disk."""
        logger.info("[MemoryStore] Saving memory store to: %s", self.filepath)
        self.filepath.parent.mkdir(parents=True, exist_ok=True)
        self.tmp_filepath.write_text(self.model_dump_json(exclude={"filepath"}), encoding="utf-8")
        self.tmp_filepath.replace(self.filepath)

    def load(self) -> None:
        """Load the memory store from disk."""
        if not self.filepath.exists():
            logger.info("[MemoryStore] No memory store found at: %s", self.filepath)
            return

        logger.info("[MemoryStore] Loading memory store from: %s", self.filepath)
        facts = json.loads(self.filepath.read_text(encoding="utf-8")).get("facts", [])

        if len(loaded_facts := facts) == 0:
            logger.info("[MemoryStore] No facts found in memory store.")

        self.facts = [Fact.model_validate(fact) for fact in loaded_facts]
        logger.info("[MemoryStore] Loaded %d facts.", len(self.facts))

    def add_fact(self, text: str, embedding: Sequence[float], similarity_threshold: float) -> None:
        """Add a new fact, avoiding exact and near-duplicate facts.

        :param str text: The text of the fact.
        :param Sequence[float] embedding: The embedding vector of the fact.
        :param float similarity_threshold: Cosine similarity above which a fact is considered a duplicate.
        """
        if any(fact.text.strip().lower() == text.strip().lower() for fact in self.facts):
            return

        if self.facts:
            existing_embeddings = [fact.embedding for fact in self.facts]
            scores = self._cosine_similarity(query=embedding, facts=existing_embeddings)
            if scores.max() >= similarity_threshold:
                return

        self.facts.append(
            Fact(text=text, embedding=embedding, created_at=datetime.datetime.now(datetime.UTC).isoformat())
        )
        logger.info("[MemoryStore] Stored new fact of length %d.", len(text))

    def retrieve(
        self, query_embedding: Sequence[float], top_k: int, min_similarity: float, max_facts: int
    ) -> list[str]:
        """Retrieve the most relevant facts for a given query embedding.

        :param Sequence[float] query_embedding: The embedding vector of the query.
        :param int top_k: The number of top similar facts to consider.
        :param float min_similarity: The minimum similarity threshold for retrieving facts.
        :param int max_facts: The maximum number of facts to retrieve.
        :return: A list of relevant fact texts.
        :rtype: list[str]
        """
        if not self.facts:
            return []

        scores = self._cosine_similarity(query=query_embedding, facts=[fact.embedding for fact in self.facts])
        ranked_indices = np.argsort(scores)[::-1]

        scored = [(float(scores[i]), self.facts[i].text) for i in ranked_indices]
        return [text for score, text in scored[:top_k] if score >= min_similarity][:max_facts]


def get_extraction_prompt(user_input: str, assistant_response: str, known_facts: list[str]) -> str:
    """Generate the extraction prompt for the LLM.

    :param str user_input: The user's message.
    :param str assistant_response: The assistant's full response.
    :param list[str] known_facts: Facts already stored in memory, to avoid re-extracting.
    :return: The extraction prompt.
    :rtype: str
    """
    known_block = (
        "Facts already known about the user (do NOT extract these again, even if reworded, "
        "elaborated on, or described from a different angle — if the new information is just "
        "a different way of describing something already known, skip it):\n"
        + "\n".join(f"- {fact}" for fact in known_facts)
        + "\n\n"
        if known_facts
        else ""
    )
    return EXTRACTION_PROMPT + known_block + f"User: {user_input}\nAssistant: {assistant_response}"
