#!/bin/bash
# Download ML models for Pi-Bot
# Reads config.yaml and downloads the specified models automatically
# Models are too large to commit to Git, so they must be downloaded separately
#
# Usage:
#   1. Edit config/config.yaml and uncomment your desired Vosk model
#   2. Run: ./scripts/download_models.sh
#   3. The script will download only the model specified in config
#
# To switch models:
#   - Comment out old model, uncomment new model in config.yaml
#   - Run this script again to download the new model
#   - Old models remain on disk (delete manually to save space)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
MODELS_DIR="$PROJECT_ROOT/models"
VOSK_DIR_BASE="$MODELS_DIR/vosk"
PIPER_DIR_BASE="$MODELS_DIR/piper"
CONFIG_FILE="$PROJECT_ROOT/config/config.yaml"

echo "================================================"
echo "Pi-Bot Model Downloader"
echo "================================================"
echo ""

# Check if config exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "✗ Config file not found: $CONFIG_FILE"
    exit 1
fi

# Create models directory structure
mkdir -p "$VOSK_DIR_BASE"
mkdir -p "$PIPER_DIR_BASE"

echo "Reading configuration from: config/config.yaml"
echo "Models will be downloaded to: $MODELS_DIR"
echo ""

# ========================================
# Vosk Speech Recognition Model
# ========================================
echo "1. Downloading Vosk Model from config"
echo ""

# Extract model path from config (get the active uncommented line)
VOSK_MODEL_PATH=$(grep -E '^[[:space:]]*model_path:' "$CONFIG_FILE" | grep 'vosk' | head -1 | sed 's/.*"\([^"]*\)".*/\1/' | sed 's|models/vosk/||')

if [ -z "$VOSK_MODEL_PATH" ]; then
    echo "   ✗ Could not find Vosk model_path in config.yaml"
    exit 1
fi

VOSK_MODEL="$VOSK_MODEL_PATH"
echo "   Model from config: $VOSK_MODEL"

# Determine size for display
case "$VOSK_MODEL" in
    *"small"*)
        SIZE_INFO="~40MB, fast, less accurate"
        ;;
    *"lgraph"*)
        SIZE_INFO="~128MB, good balance"
        ;;
    *"0.22")
        SIZE_INFO="~1.8GB, very accurate, high RAM"
        ;;
    *"gigaspeech"*)
        SIZE_INFO="~2.3GB, best accuracy"
        ;;
    *"daanzu"*)
        SIZE_INFO="~1GB, good for commands"
        ;;
    *)
        SIZE_INFO="size varies"
        ;;
esac

echo "   Model specs: $SIZE_INFO"
echo ""

VOSK_URL="https://alphacephei.com/vosk/models/${VOSK_MODEL}.zip"
VOSK_ZIP="$VOSK_DIR_BASE/${VOSK_MODEL}.zip"
VOSK_DIR="$VOSK_DIR_BASE/${VOSK_MODEL}"

if [ -d "$VOSK_DIR" ]; then
    echo "   ✓ Vosk model already exists at: $VOSK_DIR"
    echo "   To re-download, delete the directory first"
else
    echo "   Downloading from: $VOSK_URL"
    echo "   This may take several minutes depending on model size..."
    echo ""

    wget -O "$VOSK_ZIP" "$VOSK_URL" || {
        echo "   ✗ Failed to download Vosk model"
        echo "   "
        echo "   Available models: https://alphacephei.com/vosk/models"
        echo "   Make sure the model name in config.yaml is correct"
        exit 1
    }

    echo "   Extracting model..."
    cd "$VOSK_DIR_BASE"
    unzip -q "$VOSK_ZIP"
    rm "$VOSK_ZIP"

    echo "   ✓ Vosk model installed successfully"
fi

echo ""

# ========================================
# Piper TTS Model
# ========================================
echo "2. Downloading Piper TTS Model from config"
echo ""

# Extract voice from config (find voice: line in piper section, extract quoted value)
PIPER_VOICE=$(grep -E '^[[:space:]]*voice:' "$CONFIG_FILE" | grep -v '^#' | head -1 | sed 's/.*"\([^"]*\)".*/\1/')

if [ -z "$PIPER_VOICE" ]; then
    echo "   ✗ Could not find Piper voice in config.yaml"
    exit 1
fi

echo "   Voice from config: $PIPER_VOICE"
echo ""

# Parse voice to construct URL (format: en_GB-alba-medium)
LANG=$(echo "$PIPER_VOICE" | cut -d'-' -f1)  # en_GB
VOICE_NAME=$(echo "$PIPER_VOICE" | cut -d'-' -f2)  # alba
QUALITY=$(echo "$PIPER_VOICE" | cut -d'-' -f3)  # medium
LANG_SHORT=$(echo "$LANG" | cut -d'_' -f1)  # en

PIPER_BASE_URL="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/${LANG_SHORT}/${LANG}/${VOICE_NAME}/${QUALITY}"
PIPER_ONNX="$PIPER_DIR_BASE/${PIPER_VOICE}.onnx"
PIPER_JSON="$PIPER_DIR_BASE/${PIPER_VOICE}.onnx.json"

if [ -f "$PIPER_ONNX" ] && [ -f "$PIPER_JSON" ]; then
    echo "   ✓ Piper model already exists"
else
    echo "   Downloading ONNX model..."
    wget -O "$PIPER_ONNX" "${PIPER_BASE_URL}/${PIPER_VOICE}.onnx" || {
        echo "   ✗ Failed to download Piper model"
        exit 1
    }

    echo "   Downloading config..."
    wget -O "$PIPER_JSON" "${PIPER_BASE_URL}/${PIPER_VOICE}.onnx.json" || {
        echo "   ✗ Failed to download Piper config"
        exit 1
    }

    echo "   ✓ Piper model installed successfully"
fi

echo ""
echo "================================================"
echo "✓ All models downloaded successfully!"
echo "================================================"
echo ""
echo "Model locations:"
echo "  Vosk:  $VOSK_DIR"
echo "  Piper: $PIPER_DIR_BASE/"
echo ""
echo "You can now run the system with:"
echo "  cargo run --bin runner"
echo ""
