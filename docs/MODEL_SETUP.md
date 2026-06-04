# Model Setup

This project uses large ML models for offline speech recognition and text-to-speech. These models are **not** included in the Git repository due to their size.

## Quick Setup

**1. Choose your Vosk model in `config/config.yaml`:**

Uncomment the model you want to use (see model comparison below):

```yaml
vosk:
  # Uncomment one of these:
  # model_path: "models/vosk/vosk-model-small-en-us-0.15"      # 40MB - Fast, low RAM
  model_path: "models/vosk/vosk-model-en-us-0.22-lgraph"      # 128MB - Recommended
  # model_path: "models/vosk/vosk-model-en-us-0.22"            # 1.8GB - Highest accuracy
```

**2. Run the automated download script:**

```bash
./scripts/download_models.sh
```

The script reads your config and downloads only the selected models.

---

## Vosk Model Comparison

Choose based on your Raspberry Pi's RAM and accuracy needs:

| Model | Size | RAM Usage | Accuracy | Speed | Best For |
|-------|------|-----------|----------|-------|----------|
| **vosk-model-small-en-us-0.15** | 40MB | ~200MB | Low | Very Fast | Testing, low-RAM systems |
| **vosk-model-en-us-0.22-lgraph** ⭐ | 128MB | ~500MB | Good | Fast | **Recommended balance** |
| **vosk-model-en-us-0.22** | 1.8GB | ~2GB | Excellent | Moderate | Best accuracy, high RAM |
| **vosk-model-en-us-0.42-gigaspeech** | 2.3GB | ~2.5GB | Best | Slower | Ultimate accuracy |
| **vosk-model-en-us-daanzu-20200905** | 1GB | ~1.5GB | Good | Fast | Voice commands, dictation |

**💡 Recommendation**: Start with `vosk-model-en-us-0.22-lgraph` for the best balance of accuracy and RAM usage.

---

## Switching Models

1. Comment out current model in `config/config.yaml`
2. Uncomment desired model
3. Run `./scripts/download_models.sh` to download if needed
4. Restart the application

Old model files remain in `models/vosk/` and can be deleted manually to save disk space.

---

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
