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
| **RFID Reader (RC522)** | Lock/unlock bot, user identification | Phase 1 |
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
| **Ollama** | Local LLM inference | Qwen2.5 7B (fits comfortably in 16GB RAM) |
| **Piper** | Text-to-speech synthesis | `en_US-lessac-medium` voice |
| **Vosk** | Wake word detection + STT | `vosk-model-small-en-us-0.15` (40MB, offline) |

### Core Libraries & Dependencies

**Rust Crates:**
- `tokio` - Async runtime
- `rppal` - Raspberry Pi GPIO/PWM/I2C
- `serde` / `serde_yaml` - Configuration management
- `vosk` - Wake word detection + STT (offline, open source)
- `reqwest` - HTTP client for Ollama API
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
- Ollama models: ~5GB (Qwen2.5 7B)
- Vosk model: ~40MB
- Memory database: <100MB
- Logs and sessions: ~1GB over time

---

## Development Phases

### Phase 1: Basic Conversation (MVP)
**Goal**: Functional voice assistant with personality

**Components**:
- ✅ Wake word detection
- ✅ Speech-to-text
- ✅ LLM conversation (Ollama)
- ✅ Text-to-speech
- ✅ RGB LED state indication
- ✅ Green status LEDs (active states)
- ✅ Red status LEDs (idle/error states)
- ✅ PIR presence detection
- ✅ RFID lock/unlock functionality
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
- **LLM response**: <3s (Qwen2.5 7B with 16GB RAM)
- **TTS synthesis**: <1s
- **End-to-end interaction**: <6s (wake word → spoken response)

### Resource Usage
- **Idle RAM**: ~8GB (Ollama model loaded)
- **Active RAM**: ~10GB (during inference)
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
- **Network isolation**: Can operate without internet (except Ollama model download)
- **Memory review**: User can inspect/delete memory files
- **Wake word privacy**: Always-on listening only for wake phrase (Vosk keyword spotting)

---

## Getting Started

1. **Hardware Setup**: Wire components according to GPIO pin configuration
2. **Software Installation**: Install Rust, Python, Ollama, Piper
3. **Component Testing**: Run individual test binaries to verify hardware
4. **System Integration**: Build and run full Pi Bot system
5. **Personality Training**: Interact and let the bot learn about you

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed build and deployment instructions.
