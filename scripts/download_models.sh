#!/bin/bash
# Download ML models for Pi-Bot
# Models are too large to commit to Git, so they must be downloaded separately

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
MODELS_DIR="$PROJECT_ROOT/models"
VOSK_DIR_BASE="$MODELS_DIR/vosk"
PIPER_DIR_BASE="$MODELS_DIR/piper"

echo "================================================"
echo "Pi-Bot Model Downloader"
echo "================================================"
echo ""

# Create models directory structure
mkdir -p "$VOSK_DIR_BASE"
mkdir -p "$PIPER_DIR_BASE"

echo "Models will be downloaded to: $MODELS_DIR"
echo ""

# ========================================
# Vosk Speech Recognition Model
# ========================================
echo "1. Downloading Vosk Model (Full - 1.8GB)"
echo "   This provides high-accuracy offline speech recognition"
echo ""

VOSK_MODEL="vosk-model-en-us-0.22"
VOSK_URL="https://alphacephei.com/vosk/models/${VOSK_MODEL}.zip"
VOSK_ZIP="$VOSK_DIR_BASE/${VOSK_MODEL}.zip"
VOSK_DIR="$VOSK_DIR_BASE/${VOSK_MODEL}"

if [ -d "$VOSK_DIR" ]; then
    echo "   ✓ Vosk model already exists at: $VOSK_DIR"
    echo "   To re-download, delete the directory first"
else
    echo "   Downloading from: $VOSK_URL"
    echo "   This may take several minutes (1.8GB)..."

    wget -O "$VOSK_ZIP" "$VOSK_URL" || {
        echo "   ✗ Failed to download Vosk model"
        echo "   You can manually download from: https://alphacephei.com/vosk/models"
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
echo "2. Downloading Piper TTS Model (~100MB)"
echo "   This provides offline text-to-speech synthesis"
echo ""

PIPER_VOICE="en_GB-alba-medium"
PIPER_BASE_URL="https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_GB/alba/medium"
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
