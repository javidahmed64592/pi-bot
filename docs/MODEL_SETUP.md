# Model Setup

This project uses large ML models for offline speech recognition and text-to-speech. These models are **not** included in the Git repository due to their size.

## Quick Setup

Run the automated download script:

```bash
./scripts/download_models.sh
```

This will download:
- **Vosk** (1.8GB): Full English speech recognition model
- **Piper** (~100MB): British English text-to-speech model

## Manual Setup

If the script fails or you prefer manual installation:

### Vosk Model

```bash
cd models/vosk
wget https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip
unzip vosk-model-en-us-0.22.zip
rm vosk-model-en-us-0.22.zip
```

Browse available models: https://alphacephei.com/vosk/models

### Piper Model

```bash
cd models/piper
wget https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_GB/alba/medium/en_GB-alba-medium.onnx
wget https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_GB/alba/medium/en_GB-alba-medium.onnx.json
```

Browse available voices: https://huggingface.co/rhasspy/piper-voices

## Expected Directory Structure

```
models/
├── vosk/
│   └── vosk-model-en-us-0.22/
│       ├── am/
│       ├── conf/
│       ├── graph/
│       └── ...
└── piper/
    ├── en_GB-alba-medium.onnx
    └── en_GB-alba-medium.onnx.json
```

## Troubleshooting

**Different language/accent needed**
- Edit `scripts/download_models.sh` with your preferred model
- Update `config/config.yaml` to match the new model path
