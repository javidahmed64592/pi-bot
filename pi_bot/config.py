"""Configuration management for the Pi Bot."""

from pathlib import Path

from pyhere import here

from pi_bot.models import BotConfig

CONFIG_DIRECTORY = Path(here("config"))
DATA_DIRECTORY = Path(here("data"))
MODELS_DIRECTORY = Path(here("models"))

CONFIG_FILENAME = "config.yaml"
CONFIG_PATH = CONFIG_DIRECTORY / CONFIG_FILENAME


def load_config() -> BotConfig:
    """Load the bot configuration from the YAML file."""
    return BotConfig.from_yaml(CONFIG_PATH)
