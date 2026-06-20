"""LLM generation for the bot."""

from __future__ import annotations

import logging
import re
from collections.abc import Callable, Generator, Sequence
from datetime import UTC, datetime
from typing import Any

from ollama import Client
from pydantic import ValidationError

from pi_bot.llm.memory import (
    EXTRACTOR_INSTRUCTIONS,
    ExtractedFacts,
    MemoryStore,
    Message,
    MessageList,
    get_extraction_prompt,
)
from pi_bot.models import BotConfig

logger = logging.getLogger(__name__)


class Chatbot:
    """A chatbot that uses the Ollama API to generate responses."""

    def __init__(
        self,
        ollama_host: str,
        model_name: str,
        temperature: float,
        fact_retrieval_temperature: float,
        max_context_length: int,
        num_predict: int,
        max_history: int,
        system_prompt: str,
        embeddings_model_name: str,
        embeddings_temperature: float,
        top_k: int,
        add_similarity_threshold: float,
        retrieve_similarity_threshold: float,
        max_facts: int,
        tools: list[Callable[[Any], str]],
    ) -> None:
        """Initialize the chatbot with the given parameters.

        :param str ollama_host: The host URL for the Ollama API.
        :param str model_name: The name of the model to use for generation.
        :param float temperature: The temperature for generation.
        :param float fact_retrieval_temperature: The temperature for fact retrieval.
        :param int max_context_length: The maximum context length for the model.
        :param int num_predict: The number of predictions to generate.
        :param int max_history: The maximum number of messages to keep in history.
        :param str system_prompt: The system prompt to use for the chatbot.
        :param str embeddings_model_name: The name of the embeddings model to use.
        :param float embeddings_temperature: The temperature for the embeddings model.
        :param int top_k: The number of top similar facts to retrieve.
        :param float add_similarity_threshold: The similarity threshold for adding new facts to avoid duplicates.
        :param float retrieve_similarity_threshold: The similarity threshold for retrieving facts.
        :param int max_facts: The maximum number of facts to retrieve.
        :param list[Callable[[Any], str]] tools: A list of tool functions that the chatbot can use.
        """
        if "localhost" in ollama_host:
            logger.info("[%s] Using LOCAL Ollama host.", self.label)
        else:
            logger.info("[%s] Using REMOTE Ollama host.", self.label)

        self.client = Client(host=ollama_host)
        logger.info("[%s] Initialized Ollama client.", self.label)

        # LLM parameters
        self.model_name = model_name
        self.temperature = temperature
        self.fact_retrieval_temperature = fact_retrieval_temperature
        self.max_context_length = max_context_length
        self.num_predict = num_predict

        # Embeddings parameters
        self.embeddings_model_name = embeddings_model_name
        self.embeddings_temperature = embeddings_temperature
        self.top_k = top_k
        self.add_similarity_threshold = add_similarity_threshold
        self.retrieve_similarity_threshold = retrieve_similarity_threshold
        self.max_facts = max_facts

        # Tools
        self.tools = tools

        self.messages = MessageList(
            system_message=Message.system_message(content=system_prompt), max_history=max_history
        )
        self.messages.load()

        self.memory = MemoryStore()
        self.memory.load()

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

    def _embed(self, inputs: list[str]) -> Sequence[Sequence[float]]:
        """Generate embeddings for a list of texts.

        :param list[str] inputs: Texts to embed.
        :return: List of embedding vectors, one per input.
        :rtype: Sequence[Sequence[float]]
        """
        response = self.client.embed(
            model=self.embeddings_model_name,
            input=inputs,
        )
        return response.embeddings

    def _retrieve_relevant_facts(self, user_input: str) -> list[str]:
        """Embed user input and retrieve the most relevant stored facts.

        :param str user_input: The user's message.
        :return: List of relevant fact strings.
        :rtype: list[str]
        """
        if not self.memory.facts:
            return []

        query_embedding = self._embed([user_input])[0]
        return self.memory.retrieve(
            query_embedding=query_embedding,
            top_k=self.top_k,
            min_similarity=self.retrieve_similarity_threshold,
            max_facts=self.max_facts,
        )

    def _extract_and_store_facts(self, user_input: str, assistant_response: str, known_facts: list[str]) -> None:
        """Ask the LLM to extract learnable facts from the exchange and store them.

        :param str user_input: The user's message.
        :param str assistant_response: The assistant's full response.
        :param list[str] known_facts: Facts already known to the bot to avoid duplicates.
        """
        messages = MessageList(
            messages=[
                Message.user_message(
                    get_extraction_prompt(
                        user_input=user_input,
                        assistant_response=assistant_response,
                        known_facts=known_facts,
                    )
                )
            ],
            system_message=Message.system_message(content=EXTRACTOR_INSTRUCTIONS),
            max_history=2,
        )
        response = self.client.chat(
            model=self.model_name,
            messages=messages.history_dump,
            stream=False,
            format=ExtractedFacts.model_json_schema(),
            options={"temperature": self.fact_retrieval_temperature, "num_predict": 200},
        )

        if not (response_content := response.message.content):
            logger.warning("[%s] No content returned from fact extraction.", self.label)
            return

        try:
            extracted = ExtractedFacts.model_validate_json(response_content)
        except ValidationError:
            logger.warning("[%s] Failed to parse fact extraction response: %s", self.label, response_content)
            return

        if not (new_facts := [fact.strip() for fact in extracted.facts if fact.strip()]):
            logger.info("[%s] No new facts extracted.", self.label)
            return

        logger.info("[%s] Extracted %d new facts from exchange.", self.label, len(new_facts))

        embeddings = self._embed(new_facts)
        for text, embedding in zip(new_facts, embeddings, strict=False):
            self.memory.add_fact(text=text, embedding=embedding, similarity_threshold=self.add_similarity_threshold)

        self.memory.save()

    def _create_memory_block(self, facts: list[str]) -> str:
        """Create a memory block string from a list of facts.

        :param list[str] facts: List of relevant facts.
        :return: Formatted memory block string.
        :rtype: str
        """
        logger.info("[%s] Retrieved %d relevant facts for observation.", self.label, len(facts))
        return "\n\nRelevant things you know about the user:\n" + "\n".join(f"- {fact}" for fact in facts)

    @staticmethod
    def _get_current_time_block() -> str:
        """Return a formatted string with the current UTC time for system prompt injection."""
        now = datetime.now(UTC)
        return (
            f"\n\nThe current UTC time is exactly {now.strftime('%H:%M')} "
            f"on {now.strftime('%A, %d %B %Y')}. "
            f"Use this time precisely when asked - do not estimate or hallucinate it."
        )

    def _get_augmented_system_message(self, memory_block: str) -> Message:
        """Return a copy of the system message with the memory block appended.

        :param str memory_block: The memory block to append.
        :return: A new Message object with the augmented content.
        :rtype: Message
        """
        augmented_system = self.messages.system_message.model_copy()
        augmented_system.content += self._get_current_time_block()
        if memory_block:
            augmented_system.content += memory_block
        return augmented_system

    def _build_observation_context(self, minutes_at_desk: int, minutes_since_interaction: int) -> str:
        """Build the observation context prompt for proactive conversation.

        :param int minutes_at_desk: Minutes the user has been present at the desk.
        :param int minutes_since_interaction: Minutes since the last interaction.
        :return: The observation context prompt.
        :rtype: str
        """
        return (
            "You are proactively starting a conversation with the user.\n"
            f"User has been at their desk for approximately {minutes_at_desk} minutes.\n"
            f"It has been approximately {minutes_since_interaction} minutes since your last interaction.\n\n"
            "Generate a short, natural conversation opener based on this context. "
            "It could be an observation, a question, a comment about the time of day, "
            "or anything that feels organic given what you know about the user. "
            "Keep it brief - one or two sentences at most."
        )

    def chat(self, user_input: str) -> Generator[str]:
        """Generate a response from the chatbot given user input.

        :param str user_input: The user input to send to the chatbot.
        :return: A generator yielding chunks of the chatbot's response.
        :rtype: Generator[str]
        """
        logger.info("[%s] Sending message to chatbot...", self.label)
        try:
            relevant_facts = self._retrieve_relevant_facts(user_input=user_input)
            memory_block = self._create_memory_block(relevant_facts) if relevant_facts else ""
            system_message = self._get_augmented_system_message(memory_block)

            user_message = Message.user_message(content=user_input)

            messages = MessageList(
                messages=[*self.messages.messages, user_message],
                system_message=system_message,
                max_history=self.messages.max_history,
            )

            stream = self.client.chat(
                model=self.model_name,
                messages=messages.history_dump,
                tools=self.tools,
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
            self.messages.add_message(message=user_message)
            self.messages.add_message(message=assistant_message)
            self.messages.save()

            self._extract_and_store_facts(
                user_input=user_input,
                assistant_response=content,
                known_facts=relevant_facts,
            )

    def observe(self, minutes_at_desk: int, minutes_since_interaction: int) -> Generator[str]:
        """Generate a proactive conversation opener based on environmental context.

        :param int minutes_at_desk: Minutes the user has been present at the desk.
        :param int minutes_since_interaction: Minutes since the last interaction.
        :return: A generator yielding sentences of the proactive message.
        :rtype: Generator[str]
        """
        logger.info("[%s] Generating observation...", self.label)
        try:
            observation_context = self._build_observation_context(
                minutes_at_desk=minutes_at_desk,
                minutes_since_interaction=minutes_since_interaction,
            )

            relevant_facts = self._retrieve_relevant_facts(user_input=observation_context)
            memory_block = self._create_memory_block(relevant_facts) if relevant_facts else ""
            system_message = self._get_augmented_system_message(memory_block)

            observation_message = Message.user_message(content=observation_context)

            messages = MessageList(
                messages=[*self.messages.messages, observation_message],
                system_message=system_message,
                max_history=self.messages.max_history,
            )

            stream = self.client.chat(
                model=self.model_name,
                messages=messages.history_dump,
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
            logger.exception("[%s] Error during observation generation!", self.label)
            raise
        else:
            self.messages.add_message(message=assistant_message)
            self.messages.save()


def debug(config: BotConfig) -> None:
    """Debug the chatbot by printing the system message and a sample user input."""
    chatbot = Chatbot(
        ollama_host=config.llm.ollama_host,
        model_name=config.llm.model_name,
        temperature=config.llm.temperature,
        fact_retrieval_temperature=config.llm.fact_retrieval_temperature,
        max_context_length=config.llm.max_context_length,
        num_predict=config.llm.num_predict,
        max_history=config.llm.max_history,
        system_prompt=config.llm.system_prompt,
        embeddings_model_name=config.llm.embeddings.model_name,
        embeddings_temperature=config.llm.embeddings.temperature,
        top_k=config.llm.embeddings.top_k,
        add_similarity_threshold=config.llm.embeddings.add_similarity_threshold,
        retrieve_similarity_threshold=config.llm.embeddings.retrieve_similarity_threshold,
        max_facts=config.llm.embeddings.max_facts,
        tools=[],
    )

    try:
        # Make passive observation and greet user
        for chunk in chatbot.observe(minutes_at_desk=30, minutes_since_interaction=15):
            print(chunk, end="\n", flush=True)

        # Conversation loop
        while True:
            message = str(input("User: "))
            for chunk in chatbot.chat(user_input=message):
                print(chunk, end="\n", flush=True)
    except KeyboardInterrupt:
        logger.info("Chatbot debug stopped by user.")
