# Requirements for Pi Bot

The Pi Bot project aims to create an intelligent, interactive AI companion using a Raspberry Pi as the core hardware platform. The bot will be equipped with various sensors and actuators to enable environmental awareness, natural conversation, and dynamic interaction capabilities.

**For detailed information, see:**
- [FEATURES.md](./FEATURES.md) - Comprehensive feature documentation, behaviors, and interaction scenarios
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Software architecture, Rust/Python integration, and testing strategy

---

## Hardware Specifications

### Core Platform
- **Raspberry Pi 5** (16GB RAM) + active cooler
- **256GB microSD card** (high-speed, Class 10 or better)
- **USB-C power supply** (5V/5A recommended for peripherals)
- Breadboard and jumper wires for prototyping

### Sensors (Environmental Awareness)

| Component | Purpose | Phase |
|-----------|---------|-------|
| **Pi Camera Module v3** | Vision for human detection and environmental observation | Phase 2 |
| **USB Microphone** | Audio input for wake word detection and speech-to-text | Phase 1 |
| **PIR Motion Sensor** | Presence detection | Phase 1 |
| **DHT11 Sensor** | Temperature and humidity monitoring | Phase 2 |

### Actuators (Expression & Output)

| Component | Purpose | Phase |
|-----------|---------|-------|
| **RGB LED** | Primary visual expression (state indication, patterns, ambient lighting) | Phase 1 |
| **Speaker + Amplifier** | Voice output via text-to-speech | Phase 1 |
| **2x Green LEDs** | Active state indicators (ready, listening, processing) | Phase 1 |
| **2x Red LEDs** | Idle/error state indicators (idle, DND mode, system errors) | Phase 1 |
| **LCD Display (16x2)** | Text output for status and messages | Phase 2 |

### Electronic Components

- **5x 220Ω resistors** - Current limiting for LEDs
- **10kΩ resistor** - Pull-up for DHT11 sensor
- **Breadboard** - Prototyping connections
- **Jumper wires** - GPIO connections (male-to-female, male-to-male)

---

## Software Requirements

### Operating System
- **Raspberry Pi OS (64-bit)** - Bookworm or later
  - SSH enabled for remote access

### Programming Languages & Runtimes
- **Rust** (latest stable) - Primary system language
  - `cargo` for package management
  - `rustc` 1.75+
- **Python 3.13+** - For hardware interfacing (DHT11, camera helper scripts)
  - `uv` for Python package management

### AI & Machine Learning

| Component | Purpose | Model/Config |
|-----------|---------|--------------|
| **Llamafile** | Local LLM inference (OpenAI-compatible API) | Qwen2.5 3B Q4_K_M (~2GB) |
| **Piper** | Text-to-speech synthesis | `en_GB-alba-medium` voice |
| **Vosk** | Wake word detection + STT | See model comparison below |

#### Model Setup & Installation

These large ML models are **not** included in the Git repository due to their size and must be downloaded separately.

**Quick Setup:**

1. **Choose your Vosk model in `config/config.yaml`:**

   Uncomment the model you want to use:

   ```yaml
   vosk:
     # Uncomment one of these:
     # model_path: "models/vosk/vosk-model-small-en-us-0.15"      # 40MB - Fast, low RAM
     model_path: "models/vosk/vosk-model-en-us-0.22-lgraph"      # 128MB - Recommended
     # model_path: "models/vosk/vosk-model-en-us-0.22"            # 1.8GB - Highest accuracy
   ```

2. **Run the automated download script:**

   ```bash
   ./scripts/download_models.sh
   ```

   The script reads your config and downloads only the selected models.

#### Vosk Model Comparison

Choose based on your Raspberry Pi's RAM and accuracy needs:

| Model | Size | RAM Usage | Accuracy | Speed | Best For |
|-------|------|-----------|----------|-------|----------|
| **vosk-model-small-en-us-0.15** | 40MB | ~200MB | Low | Very Fast | Testing, low-RAM systems |
| **vosk-model-en-us-0.22-lgraph** ⭐ | 128MB | ~500MB | Good | Fast | **Recommended balance** |
| **vosk-model-en-us-0.22** | 1.8GB | ~2GB | Excellent | Moderate | Best accuracy, high RAM |
| **vosk-model-en-us-0.42-gigaspeech** | 2.3GB | ~2.5GB | Best | Slower | Ultimate accuracy |
| **vosk-model-en-us-daanzu-20200905** | 1GB | ~1.5GB | Good | Fast | Voice commands, dictation |

**💡 Recommendation**: Start with `vosk-model-en-us-0.22-lgraph` for the best balance of accuracy and RAM usage.

#### Switching Models

1. Comment out current model in `config/config.yaml`
2. Uncomment desired model
3. Run `./scripts/download_models.sh` to download if needed
4. Restart the application

Old model files remain in `models/vosk/` and can be deleted manually to save disk space.

#### Manual Setup

If the automated script fails or you prefer manual installation:

**Vosk Model:**
```bash
cd models/vosk
wget https://alphacephei.com/vosk/models/vosk-model-en-us-0.22-lgraph.zip
unzip vosk-model-en-us-0.22-lgraph.zip
rm vosk-model-en-us-0.22-lgraph.zip
```

Browse available models: https://alphacephei.com/vosk/models

**Piper Model:**
```bash
cd models/piper
wget https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_GB/alba/medium/en_GB-alba-medium.onnx
wget https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_GB/alba/medium/en_GB-alba-medium.onnx.json
```

Browse available voices: https://huggingface.co/rhasspy/piper-voices

**Llamafile Model:**
```bash
cd models/llamafile
# Download llamafile executable
wget https://github.com/Mozilla-Ocho/llamafile/releases/download/0.8.13/llamafile-0.8.13
chmod +x llamafile-0.8.13
mv llamafile-0.8.13 llamafile

# Download Qwen2.5 3B model (Q4_K_M quantization, ~2GB)
wget https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf
```

Browse available models: https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF

**Expected Directory Structure:**
```
models/
├── vosk/
│   └── vosk-model-en-us-0.22-lgraph/
│       ├── am/
│       ├── conf/
│       ├── graph/
│       └── ...
├── piper/
│   ├── en_GB-alba-medium.onnx
│   └── en_GB-alba-medium.onnx.json
└── llamafile/
    ├── llamafile                          # Executable
    └── qwen2.5-3b-instruct-q4_k_m.gguf   # Model weights
```

**Troubleshooting:**
- **Different language/accent needed**: Edit `scripts/download_models.sh` or update `config/config.yaml` to match your preferred model

#### Llamafile Setup

**Why Llamafile?**

Llamafile offers significant advantages for single-board computers like Raspberry Pi:
- **Single executable**: No separate model management tools required
- **Better performance**: ~20-30% faster inference on ARM architectures
- **Lower memory footprint**: More efficient resource usage (~500MB less than Ollama)
- **OpenAI-compatible API**: Standard interface for easy integration
- **Simple deployment**: Download and run—no installation required

**Running Llamafile:**

After downloading models with `./scripts/download_models.sh`, start the llamafile server:

```bash
# Start llamafile server (from project root)
./models/llamafile/llamafile --server --port 8080 \
  -m ./models/llamafile/qwen2.5-3b-instruct-q4_k_m.gguf \
  --host 0.0.0.0

# Or run in background
nohup ./models/llamafile/llamafile --server --port 8080 \
  -m ./models/llamafile/qwen2.5-3b-instruct-q4_k_m.gguf \
  --host 0.0.0.0 > llamafile.log 2>&1 &
```

**Systemd Service (Auto-start on Boot):**

Create `/etc/systemd/system/llamafile.service`:

```ini
[Unit]
Description=Llamafile LLM Server for Pi Bot
After=network.target

[Service]
Type=simple
User=javid
WorkingDirectory=/home/javid/Code/pi-bot
ExecStart=/home/javid/Code/pi-bot/models/llamafile/llamafile --server --port 8080 -m /home/javid/Code/pi-bot/models/llamafile/qwen2.5-3b-instruct-q4_k_m.gguf --host 0.0.0.0
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable llamafile
sudo systemctl start llamafile
sudo systemctl status llamafile
```

**Performance Tuning:**

- **Model quantization options**:
  - Q4_K_M (recommended): ~2GB RAM, good balance
  - Q5_K_M: ~3GB RAM, better quality
  - Q3_K_M: ~1.5GB RAM, lower quality but faster

- **Thread configuration**:
  ```bash
  ./llamafile --server --port 8080 -m model.gguf --threads 4
  ```

- **Expected performance on Pi 5**: 5-15 tokens/second with Q4_K_M

**Verification:**

Test the API is working:
```bash
# Check models endpoint
curl http://localhost:8080/v1/models

# Test completion
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen2.5-3b-instruct",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

**Troubleshooting:**
- **Different language/accent needed**: Edit `scripts/download_models.sh` with your preferred model and update `config/config.yaml` to match the new model path

### Core Libraries & Dependencies

**Rust Crates:**
- `tokio` - Async runtime
- `rppal` - Raspberry Pi GPIO/PWM/I2C
- `serde` / `serde_yaml` - Configuration management
- `vosk` - Wake word detection + STT (offline, open source)
- `reqwest` - HTTP client for LLM API (OpenAI-compatible)
- `rodio` / `cpal` - Audio playback

**Python Packages (via uv):**
- `gpiozero` - DHT11 sensor interface
- `lgpio` - Low-level GPIO for DHT11
- `picamera2` - Camera interface (optional)

### Memory & Storage Architecture

**Persistent Memory System:**
- **Short-term memory**: Last 10 conversation exchanges (RAM-based)
- **Session memory**: Current day's interactions (JSON file)
- **Long-term memory**: User preferences, learned facts, personality traits (JSON database)
- Pattern inspired by mem0 for efficient context retrieval

**Storage Requirements:**
- System files: ~20GB
- Llamafile models: ~2-3GB (Qwen2.5 3B Q4_K_M + executable)
- Vosk model: 40MB-2.3GB (depending on model choice)
- Piper TTS model: ~60MB
- Memory database: <100MB
- Logs and sessions: ~1GB over time

---

## Development Phases

### Phase 1: Basic Conversation (MVP)
**Goal**: Functional voice assistant with personality

**Components**:
- ✅ Wake word detection
- ✅ Speech-to-text
- ✅ LLM conversation (Llamafile)
- ✅ Text-to-speech
- ✅ RGB LED state indication
- ✅ Green status LEDs (active states)
- ✅ Red status LEDs (idle/error states)
- ✅ PIR presence detection
- ✅ Basic memory (conversation history)

**Deliverable**: Bot you can talk to with wake phrase, LED feedback, lock/unlock security, and personality

---

### Phase 2: Environmental Awareness
**Goal**: Bot observes environment and initiates interactions

**Components**:
- ✅ Camera vision (human detection)
- ✅ DHT11 environmental monitoring
- ✅ LCD display
- ✅ Passive observation mode
- ✅ Enhanced memory system

**Deliverable**: Bot that can observe, remember, and proactively start conversations

---

### Phase 3: Advanced Personality
**Goal**: Bot feels truly alive

**Components**:
- Emotion detection from speech
- Contextual humor and timing
- Music detection and reactive lighting
- Gesture recognition
- User behavior modeling
- Adaptive personality

**Deliverable**: Companion bot that evolves and feels like a roommate

---

## System Requirements

### Performance Targets
- **Wake word latency**: <200ms
- **STT processing**: <2s (for 5-second speech clip)
- **LLM response**: <2s (Qwen2.5 3B with llamafile)
- **TTS synthesis**: <1s
- **End-to-end interaction**: <5s (wake word → spoken response)

### Resource Usage
- **Idle RAM**: ~3GB (llamafile model loaded)
- **Active RAM**: ~4GB (during inference)
- **CPU usage**: ~40-60% during conversation
- **Storage growth**: ~50MB/day (conversation logs)

### Reliability Requirements
- **Component independence**: Any sensor/actuator can fail without system crash
- **Graceful degradation**: System operates with reduced capabilities if components fail
- **Auto-restart**: Failed components automatically retry after 5s
- **Health monitoring**: Status LEDs indicate system health
- **Clean shutdown**: All components shut down gracefully on SIGINT

---

## Security & Privacy

- **Local processing**: All AI inference runs on-device (no cloud)
- **Data ownership**: All conversation logs stored locally
- **Network isolation**: Can operate without internet (except model download)
- **Memory review**: User can inspect/delete memory files
- **Wake word privacy**: Always-on listening only for wake phrase (Vosk keyword spotting)

---

## Getting Started

1. **Hardware Setup**: Wire components according to GPIO pin configuration
2. **Software Installation**: Install Rust, Python, download models with `./scripts/download_models.sh`
3. **Start Llamafile**: Run the LLM server (see Llamafile Setup section above)
4. **Component Testing**: Run individual test binaries to verify hardware
5. **System Integration**: Build and run full Pi Bot system
6. **Personality Training**: Interact and let the bot learn about you

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed build and deployment instructions.
