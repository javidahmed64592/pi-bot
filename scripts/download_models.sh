#!/bin/bash
# Download ML models for Pi-Bot (Vosk for speech recognition, Piper for TTS, Ollama for LLM)
# Reads config.yaml and downloads the specified models automatically
# Models are too large to commit to Git, so they must be downloaded separately
#
# Usage:
#   1. Edit config/config.yaml and set your desired models
#   2. Run: ./scripts/download_models.sh

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

VOSK_MODEL=$(grep -E '^\s*model_name:' "${CONFIG_FILE}" | grep 'vosk' | head -1 | sed 's/.*"\([^"]*\)".*/\1/')

if [ -z "${VOSK_MODEL}" ]; then
    # Fall back to finding model_name under the vosk: section
    VOSK_MODEL=$(awk '/^audio:/,0' "${CONFIG_FILE}" | awk '/vosk:/,/piper:/' | grep 'model_name:' | head -1 | sed 's/.*"\([^"]*\)".*/\1/')
fi

if [ -z "${VOSK_MODEL}" ]; then
    echo -e "   ${RED}✗${NC} Could not find Vosk model_name in ${BLUE}config.yaml${NC}"
    exit 1
fi

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

PIPER_VOICE=$(awk '/^audio:/,0' "${CONFIG_FILE}" | awk '/piper:/,/^[^ ]/' | grep 'model_name:' | head -1 | sed 's/.*"\([^"]*\)".*/\1/')

if [ -z "${PIPER_VOICE}" ]; then
    echo -e "   ${RED}✗${NC} Could not find Piper model_name in ${BLUE}config.yaml${NC}"
    exit 1
fi

echo -e "   Voice from config: ${GREEN}${PIPER_VOICE}${NC}"

LANG=$(echo "${PIPER_VOICE}" | cut -d'-' -f1)
VOICE_NAME=$(echo "${PIPER_VOICE}" | cut -d'-' -f2)
QUALITY=$(echo "${PIPER_VOICE}" | cut -d'-' -f3)
LANG_SHORT=$(echo "${LANG}" | cut -d'_' -f1)

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
# Ollama Models (LLM + Embeddings)
# ========================================
echo "3. Checking Ollama Models from config"
echo

OLLAMA_HOST=$(grep -E '^  ollama_host:' "${CONFIG_FILE}" | head -1 | sed 's/[^"]*"\([^"]*\)".*/\1/')
LLM_MODEL=$(awk '/^llm:/{found=1; next} found && /^[^ ]/{exit} found && /^  model_name:/{print; exit}' "${CONFIG_FILE}" | sed 's/[^"]*"\([^"]*\)".*/\1/')
EMBEDDINGS_MODEL=$(awk '/^  embeddings:/{found=1; next} found && /^  [^ ]/{exit} found && /^    model_name:/{print; exit}' "${CONFIG_FILE}" | sed 's/[^"]*"\([^"]*\)".*/\1/')

if [ -z "${LLM_MODEL}" ]; then
    echo -e "   ${RED}✗${NC} Could not find LLM model_name in ${BLUE}config.yaml${NC}"
    exit 1
fi

if [ -z "${EMBEDDINGS_MODEL}" ]; then
    echo -e "   ${RED}✗${NC} Could not find embeddings model_name in ${BLUE}config.yaml${NC}"
    exit 1
fi

echo -e "   Ollama host:       ${BLUE}${OLLAMA_HOST}${NC}"
echo -e "   LLM model:         ${GREEN}${LLM_MODEL}${NC}"
echo -e "   Embeddings model:  ${GREEN}${EMBEDDINGS_MODEL}${NC}"
echo

# Only attempt to pull if the host is localhost
if echo "${OLLAMA_HOST}" | grep -qE '(localhost|127\.0\.0\.1)'; then
    if ! command -v ollama &> /dev/null; then
        echo -e "   ${RED}✗${NC} Ollama CLI not found. Please install Ollama first."
        echo -e "   ${BLUE}https://ollama.com/docs/installation${NC}"
        exit 1
    fi

    pull_if_missing() {
        local MODEL="$1"
        local LABEL="$2"
        if ollama list | grep -q "^${MODEL}"; then
            echo -e "   ${GREEN}✓${NC} ${LABEL} model already exists: ${GREEN}${MODEL}${NC}"
        else
            echo -e "   Pulling ${LABEL} model: ${GREEN}${MODEL}${NC}"
            ollama pull "${MODEL}" || {
                echo -e "   ${RED}✗${NC} Failed to pull ${LABEL} model: ${MODEL}"
                exit 1
            }
            echo -e "   ${GREEN}✓${NC} ${LABEL} model pulled successfully"
        fi
    }

    pull_if_missing "${LLM_MODEL}" "LLM"
    pull_if_missing "${EMBEDDINGS_MODEL}" "Embeddings"
else
    echo -e "   ${YELLOW}⚠${NC}  Ollama host is remote (${BLUE}${OLLAMA_HOST}${NC}) - skipping model download."
    echo "   Please ensure the following models are available on the remote host:"
    echo -e "     LLM:        ${GREEN}ollama pull ${LLM_MODEL}${NC}"
    echo -e "     Embeddings: ${GREEN}ollama pull ${EMBEDDINGS_MODEL}${NC}"
fi

# ========================================
# Summary
# ========================================
echo
echo "${SEPARATOR}"
echo -e "${GREEN}✓${NC} Setup complete!"
echo "${SEPARATOR}"
echo
echo "Models:"
echo -e "  Vosk:       ${GREEN}${VOSK_MODEL}${NC}"
echo -e "  Piper:      ${GREEN}${PIPER_VOICE}${NC}"
echo -e "  LLM:        ${GREEN}${LLM_MODEL}${NC}"
echo -e "  Embeddings: ${GREEN}${EMBEDDINGS_MODEL}${NC}"
echo
echo "You can now run the bot with:"
echo -e "  ${GREEN}uv run pi-bot${NC}"
