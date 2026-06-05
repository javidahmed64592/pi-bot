#!/bin/bash
# Download ML models for Pi-Bot (Vosk for speech recognition, Piper for TTS, Ollama for LLM)
# Reads config.yaml and downloads the specified models automatically
# Models are too large to commit to Git, so they must be downloaded separately
#
# Usage:
#   1. Edit config/config.yaml and set your desired models
#   2. Run: ./scripts/download_models.sh
#
# Note: Ollama must be installed and running locally to download LLM models.

set -eu

TERMINAL_WIDTH=$(tput cols 2>/dev/null || echo 80)
SEPARATOR=$(printf '=%.0s' $(seq 1 $TERMINAL_WIDTH))

NC='\033[0m'
RED='\033[1;31m'
GREEN='\033[1;32m'
YELLOW='\033[1;33m'
BLUE='\033[1;34m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "${SCRIPT_DIR}")"
MODELS_DIR="${PROJECT_ROOT}/models"
VOSK_DIR_BASE="${MODELS_DIR}/vosk"
PIPER_DIR_BASE="${MODELS_DIR}/piper"
CONFIG_FILE="${PROJECT_ROOT}/config/config.yaml"

echo "${SEPARATOR}"
echo "Pi-Bot Model Downloader"
echo "${SEPARATOR}"

# Check if config exists
if [ ! -f "${CONFIG_FILE}" ]; then
    echo -e "${RED}✗${NC} Config file not found: ${BLUE}${CONFIG_FILE}${NC}"
    exit 1
fi

# Create models directory structure
mkdir -p "${VOSK_DIR_BASE}"
mkdir -p "${PIPER_DIR_BASE}"

echo -e "Reading configuration from: ${BLUE}${CONFIG_FILE}${NC}"
echo -e "Vosk and Piper models will be downloaded to: ${BLUE}${MODELS_DIR}${NC}"
echo

# ========================================
# Vosk Speech Recognition Model
# ========================================
echo "1. Downloading Vosk Model from config"
echo

# Extract model path from config (get the active uncommented line)
VOSK_MODEL_PATH=$(grep -E '^[[:space:]]*model_path:' "${CONFIG_FILE}" | grep 'vosk' | head -1 | sed 's/.*"\([^"]*\)".*/\1/' | sed 's|models/vosk/||')

if [ -z "${VOSK_MODEL_PATH}" ]; then
    echo -e "   ${RED}✗${NC} Could not find Vosk ${GREEN}model_path${NC} in ${BLUE}config.yaml${NC}"
    exit 1
fi

VOSK_MODEL="${VOSK_MODEL_PATH}"
echo -e "   Model from config: ${GREEN}${VOSK_MODEL}${NC}"

VOSK_URL="https://alphacephei.com/vosk/models/${VOSK_MODEL}.zip"
VOSK_ZIP="${VOSK_DIR_BASE}/${VOSK_MODEL}.zip"
VOSK_DIR="${VOSK_DIR_BASE}/${VOSK_MODEL}"

if [ -d "${VOSK_DIR}" ]; then
    echo -e "   ${GREEN}✓${NC} Vosk model already exists at: ${BLUE}${VOSK_DIR}${NC}"
    echo "   To re-download, delete the directory first"
else
    echo -e "   Downloading from: ${BLUE}${VOSK_URL}${NC}"
    echo "   This may take several minutes depending on model size..."
    echo

    wget -O "${VOSK_ZIP}" "${VOSK_URL}" || {
        echo -e "   ${RED}✗${NC} Failed to download Vosk model"
        echo "   "
        echo -e "   Available models: ${BLUE}https://alphacephei.com/vosk/models${NC}"
        echo -e "   Make sure the model name in ${BLUE}config.yaml${NC} is correct"
        exit 1
    }

    echo "   Extracting model..."
    cd "${VOSK_DIR_BASE}"
    unzip -q "${VOSK_ZIP}"
    rm "${VOSK_ZIP}"

    echo -e "   ${GREEN}✓${NC} Vosk model installed successfully"
fi

echo

# ========================================
# Piper TTS Model
# ========================================
echo "2. Downloading Piper TTS Model from config"
echo

# Extract voice from config (find voice: line in piper section, extract quoted value)
PIPER_VOICE=$(grep -E '^[[:space:]]*voice:' "${CONFIG_FILE}" | grep -v '^#' | head -1 | sed 's/.*"\([^"]*\)".*/\1/')

if [ -z "${PIPER_VOICE}" ]; then
    echo -e "   ${RED}✗${NC} Could not find Piper voice in ${BLUE}config.yaml${NC}"
    exit 1
fi

echo -e "   Voice from config: ${GREEN}${PIPER_VOICE}${NC}"

# Parse voice to construct URL (format: en_GB-alba-medium)
LANG=$(echo "${PIPER_VOICE}" | cut -d'-' -f1)  # en_GB
VOICE_NAME=$(echo "${PIPER_VOICE}" | cut -d'-' -f2)  # alba
QUALITY=$(echo "${PIPER_VOICE}" | cut -d'-' -f3)  # medium
LANG_SHORT=$(echo "${LANG}" | cut -d'_' -f1)  # en

PIPER_BASE_URL="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/${LANG_SHORT}/${LANG}/${VOICE_NAME}/${QUALITY}"
PIPER_ONNX="${PIPER_DIR_BASE}/${PIPER_VOICE}.onnx"
PIPER_JSON="${PIPER_DIR_BASE}/${PIPER_VOICE}.onnx.json"

if [ -f "${PIPER_ONNX}" ] && [ -f "${PIPER_JSON}" ]; then
    echo -e "   ${GREEN}✓${NC} Piper model already exists at: ${BLUE}${PIPER_ONNX}${NC}"
else
    echo "   Downloading ONNX model..."
    wget -O "${PIPER_ONNX}" "${PIPER_BASE_URL}/${PIPER_VOICE}.onnx" || {
        echo -e "   ${RED}✗${NC} Failed to download Piper model"
        exit 1
    }

    echo "   Downloading config..."
    wget -O "${PIPER_JSON}" "${PIPER_BASE_URL}/${PIPER_VOICE}.onnx.json" || {
        echo -e "   ${RED}✗${NC} Failed to download Piper config"
        exit 1
    }

    echo -e "   ${GREEN}✓${NC} Piper model installed successfully"
fi

echo

# ========================================
# Ollama LLM Model
# ========================================
echo "3. Downloading Ollama LLM Model from config"
echo

# Check if Ollama is installed and running
if ! command -v ollama &> /dev/null; then
    echo -e "   ${RED}✗${NC} Ollama CLI not found. Please install Ollama to download LLM models."
    echo -e "   ${BLUE}https://ollama.com/docs/installation${NC}"
    exit 1
fi

# Extract LLM model from config
LLM_MODEL=$(grep -E '^[[:space:]]*model:' "${CONFIG_FILE}" | grep -v '^#' | head -1 | sed 's/.*"\([^"]*\)".*/\1/')

if [ -z "${LLM_MODEL}" ]; then
    echo -e "   ${RED}✗${NC} Could not find LLM model in ${BLUE}config.yaml${NC}"
    exit 1
fi

echo -e "   LLM model from config: ${GREEN}${LLM_MODEL}${NC}"

# Check if model is already installed in Ollama
if ollama list | grep -q "${LLM_MODEL}"; then
    echo -e "   ${GREEN}✓${NC} LLM model already exists in Ollama"
else
    echo "   Pulling LLM model from Ollama registry..."
    ollama pull "${LLM_MODEL}" || {
        echo -e "   ${RED}✗${NC} Failed to pull LLM model from Ollama"
        echo "   Make sure the model name in ${BLUE}config.yaml${NC} is correct and that Ollama is running"
        exit 1
    }
    echo -e "   ${GREEN}✓${NC} LLM model pulled successfully"
fi

# ========================================
# Summary
# ========================================
echo
echo ${SEPARATOR}
echo -e "${GREEN}✓${NC} Downloads complete!"
echo ${SEPARATOR}
echo
echo "Models:"
echo -e "  Vosk:  ${GREEN}${VOSK_MODEL}${NC}"
echo -e "  Piper: ${GREEN}${PIPER_VOICE}${NC}"
echo -e "  LLM:   ${GREEN}${LLM_MODEL}${NC}"
echo
echo "You can now run the system with:"
echo -e "  ${GREEN}cargo run --bin runner${NC}"
echo
