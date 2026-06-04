# Pi Bot - Software Architecture

## Overview

Pi Bot uses an event-driven software architecture with AI capabilities, audio processing, and intelligent decision-making. The system prioritizes **component independence**, **fault tolerance**, and **testability**.

---

## Core Architecture Principles

### 1. Event-Driven Communication

```
[Sensors] → Events → [AI Controller] → Commands → [Actuators]
                          ↓
                    [LLM Service]
                    [Memory Service]
```

**Key Rules:**
- Sensors emit events only (no direct actuator control)
- AI Controller owns all behavioral logic
- Actuators consume commands only (no business logic)
- Components communicate through async channels
- No shared mutable state between components

### 2. Component Independence

Each component is a separate async task that:
- Operates independently in its own event loop
- Has its own error handling and recovery
- Can fail without bringing down the entire system
- Reports health status to system monitor
- Can be tested in isolation

**Example**: If the LCD display fails, the bot continues speaking and showing LED patterns.

### 3. Fault Tolerance

System design for graceful degradation:
- Each component wrapped in supervisor with restart logic
- Component failures logged and reported via status LEDs
- Critical path identified (LLM + audio + RGB LED minimum viable system)
- Non-critical components (camera, LCD, DHT11) can fail without system crash
- Shutdown signal broadcasts to all tasks for clean shutdown

### 4. Testability

Every hardware component has a standalone test binary:
- Simple functionality (turn LED on/off, read sensor value)
- Minimal dependencies (just the component + print statements)
- Can be run before system integration
- Helps verify correct wiring and GPIO configuration

---

## System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         Runner (main.rs)                        │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ - Bootstrap channels                                       │ │
│  │ - Spawn component tasks                                    │ │
│  │ - Handle shutdown signal (Ctrl+C)                          │ │
│  │ - Monitor component health                                 │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
            ┌─────────────────┼─────────────────┐
            ↓                 ↓                 ↓
    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
    │   Sensors    │  │     Core     │  │  Actuators   │
    │   (Tasks)    │  │ (Controller) │  │   (Tasks)    │
    └──────────────┘  └──────────────┘  └──────────────┘
            │                 │                 │
            ↓                 ↓                 ↓
    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
    │  - PIR       │  │ AI Controller│  │  - RGB LED   │
    │  - Camera    │  │ ├─ LLM Svc   │  │  - Speaker   │
    │  - Mic       │  │ ├─ Memory    │  │  - Green LEDs│
    │  - DHT11     │  │ └─ State     │  │  - Red LEDs  │
    │              │  │              │  │  - LCD       │
    └──────────────┘  └──────────────┘  └──────────────┘

    Event Flow:
    Events ────────────────→ Controller ────────────→ Commands
    (sensor data)         (decision making)      (actuator control)
```

---

## Workspace Structure

```
pi-bot/
├── Cargo.toml                    # Workspace manifest
├── config/
│   └── config.yaml               # Hardware pins, timing, LLM config
│
├── bot_core/                     # Shared types (extends gpio_core)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── events.rs             # All event types
│       ├── commands.rs           # All command types
│       ├── state.rs              # Bot state machine
│       └── config.rs             # Config loading
│
├── sensors/                      # Hardware input controllers
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── pir_controller.rs
│       ├── camera_controller.rs  # Vision using picamera2
│       ├── audio_controller.rs   # Microphone input
│       ├── dht11_controller.rs   # Temperature/humidity sensor
│       └── bin/
│           ├── pir-test.rs
│           ├── camera-test.rs
│           ├── mic-test.rs
│           └── dht11-test.rs
│
├── actuators/                    # Hardware output controllers
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── rgb_led_controller.rs # RGB LED with PWM control
│       ├── speaker_controller.rs # TTS audio output
│       ├── status_led_controller.rs
│       ├── lcd_controller.rs     # LCD text display
│       └── bin/
│           ├── rgb-test.rs
│           ├── speaker-test.rs
│           ├── status-test.rs
│           └── lcd-test.rs
│
├── ai_controller/                # AI decision-making brain
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── controller.rs         # Main event handler
│       ├── llm_service.rs        # Ollama integration
│       ├── memory_service.rs     # Persistent memory
│       ├── state_machine.rs      # Bot behavioral modes
│       └── personality.rs        # Personality traits & response style
│
├── audio_pipeline/               # STT/TTS processing
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── wake_word.rs          # Vosk keyword spotting
│       ├── stt.rs                # Vosk full recognition
│       └── tts.rs                # Piper integration
│
├── vision_pipeline/              # Camera ML processing
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── human_detection.rs    # Presence detection
│       └── object_detection.rs   # Environmental changes
│
├── runner/                       # System orchestration
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # Bootstrap and task spawning
│       ├── pir_sensor.rs
│       ├── camera_sensor.rs
│       ├── audio_sensor.rs
│       ├── dht11_sensor.rs
│       ├── rgb_led_actuator.rs
│       ├── speaker_actuator.rs
│       ├── green_led_actuator.rs
│       ├── red_led_actuator.rs
│       ├── lcd_actuator.rs
│       └── health_monitor.rs     # Component health tracking
│
└── bot_utils/                    # Python utilities
    ├── __init__.py
    ├── dht11.py                  # DHT11 temperature/humidity reader
    └── camera_capture.py         # Camera frame capture helper
```

---

## Core Crates

### bot_core - Protocol Layer

**Purpose**: Define the communication protocol between all components

**Key Types**:

```rust
// events.rs
pub enum Event {
    // Presence & Motion
    PresenceDetected,
    NoPresenceSince(Duration),

    // Audio Events
    WakeWordDetected,
    SpeechCaptured(String),
    AmbientNoiseLevel(u8),

    // Vision Events
    HumanDetected { confidence: f32 },
    DeskOccupied,
    ObjectChange { description: String },

    // Environmental
    EnvironmentReading { temp: f32, humidity: f32 },
    ProximityChanged { distance_cm: u16 },

    // System
    ComponentHealth { component: String, healthy: bool },
}

// commands.rs
pub enum Command {
    // RGB LED
    SetColor(RgbColor),
    SetPattern { pattern: LedPattern, colors: Vec<RgbColor> },

    // Audio
    Speak { text: String, emotion: Emotion },
    StopSpeaking,

    // Status LEDs
    SetSystemHealth(HealthLevel),

    // Display
    ShowText { line1: String, line2: String },

    // AI Control
    StartListening,
    StopListening,
    EnterConversationState(ConversationState),
    SetLightingMode(LightingMode),
}

// state.rs
pub struct BotState {
    pub conversation_state: ConversationState,
    pub lighting_mode: LightingMode,
    pub presence_detected: bool,
    pub last_interaction: Instant,
    pub current_emotion: Emotion,
    pub observing_since: Option<Instant>,
    // ... other state
}

pub enum ConversationState {
    Ready,      // Default, monitoring, can observe and talk
    Observing,  // Noticed something, deciding whether to speak
    Active(ActiveSubState),  // In conversation
    Silent,     // Do Not Disturb, won't initiate
}

pub enum ActiveSubState {
    Listening,  // Capturing speech
    Thinking,   // Processing through LLM
    Speaking,   // Playing TTS
    Learning,   // Storing memory
}

pub enum LightingMode {
    StateBased,  // LED reflects conversation state
    Ambient(AmbientPattern),  // Decorative pattern
    Minimal,     // Dim or off
}

pub enum AmbientPattern {
    Gradient { colors: Vec<RgbColor>, speed: f32 },
    Rainbow { speed: f32 },
    Pulse { color: RgbColor, speed: f32 },
    Static { color: RgbColor },
}
```

**Dependencies**: `serde`, `tokio`, `rppal` (for GPIO types)

---

### sensors - Hardware Input

**Purpose**: Hardware controllers that emit events

**Pattern**:
```rust
pub struct AudioController {
    // Hardware handles
}

impl AudioController {
    pub fn new() -> Result<Self, AudioError> {
        // Initialize microphone
    }

    // Returns events, never controls actuators
    pub async fn check_for_wake_word(&mut self) -> Option<Event> {
        // Vosk keyword spotting
    }

    pub async fn capture_speech(&mut self) -> Option<Event> {
        // Vosk full recognition
    }
}
```

**Key Controllers**:
- `PirController` - Motion detection
- `CameraController` - Vision processing
- `AudioController` - Wake word + STT
- `Dht11Controller` - Environmental sensing

**Test Binaries**: Each controller has standalone test in `src/bin/`

---

### actuators - Hardware Output

**Purpose**: Hardware controllers that consume commands

**Pattern**:
```rust
pub struct SpeakerController {
    // TTS engine, audio device
}

impl SpeakerController {
    pub fn new() -> Result<Self, SpeakerError> {
        // Initialize speaker + Piper TTS
    }

    // Consumes commands, never emits events
    pub async fn speak(&mut self, text: &str, emotion: Emotion) -> Result<()> {
        // Generate and play TTS audio
    }
}
```

**Key Controllers**:
- `RgbLedController` - LED control with PWM
- `SpeakerController` - TTS audio output
- `GreenLedController` - Active state indicators (2 green LEDs)
- `RedLedController` - Idle/error state indicators (2 red LEDs)
- `LcdController` - Text display

**Test Binaries**: Each controller has standalone test in `src/bin/`

---

### ai_controller - Brain

**Purpose**: All behavioral logic and decision-making

**Structure**:
```rust
// controller.rs
pub async fn run_controller(
    mut event_rx: mpsc::Receiver<Event>,
    led_tx: mpsc::Sender<Command>,
    speaker_tx: mpsc::Sender<Command>,
    status_tx: mpsc::Sender<Command>,
    lcd_tx: mpsc::Sender<Command>,
    mut shutdown_rx: broadcast::Receiver<()>,
    config: BotConfig,
) {
    let mut state = BotState::default();
    let mut llm = LlmService::new(config.llm).await;
    let mut memory = MemoryService::new(config.memory).await;

    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                let commands = handle_event(
                    event,
                    &mut state,
                    &mut llm,
                    &mut memory,
                    &config
                ).await;

                for cmd in commands {
                    dispatch_command(cmd, &led_tx, &speaker_tx, &status_tx, &lcd_tx).await;
                }
            }
            _ = shutdown_rx.recv() => {
                break;
            }
        }
    }
}

async fn handle_event(
    event: Event,
    state: &mut BotState,
    llm: &mut LlmService,
    memory: &mut MemoryService,
    config: &BotConfig,
) -> Vec<Command> {
    match event {
        Event::WakeWordDetected => {
            // Transition to Active conversation
            state.conversation_state = ConversationState::Active(ActiveSubState::Listening);

            // Generate LED command based on lighting mode
            let led_cmd = match state.lighting_mode {
                LightingMode::StateBased => Command::SetPattern {
                    pattern: LedPattern::Breathing,
                    colors: vec![RgbColor::orange()],
                },
                LightingMode::Ambient(_) => {
                    // Keep ambient pattern, optionally dim slightly
                    Command::SetBrightness(80)
                }
                _ => Command::SetColor(RgbColor::orange()),
            };

            vec![led_cmd, Command::StartListening]
        }

        Event::SpeechCaptured(text) => {
            // Transition to Thinking
            state.conversation_state = ConversationState::Active(ActiveSubState::Thinking);

            // Build context
            let context = memory.get_relevant_context(&text).await;

            // Query LLM
            let response = llm.generate_response(&text, &context, state).await;

            // Update memory
            memory.store_exchange(&text, &response).await;

            // Generate LED command (only if state-based)
            let led_cmd = match state.lighting_mode {
                LightingMode::StateBased => Command::SetColor(RgbColor::blue()),
                _ => Command::NoOp, // Keep current lighting
            };

            vec![
                led_cmd,
                Command::Speak {
                    text: response,
                    emotion: state.current_emotion,
                },
            ]
        }

        Event::PresenceDetected => {
            state.presence_detected = true;

            // Only initiate observation if in Ready state (not Silent)
            if matches!(state.conversation_state, ConversationState::Ready) {
                if should_initiate_observation(state, config) {
                    state.conversation_state = ConversationState::Observing;
                    state.observing_since = Some(Instant::now());

                    // Subtle pulse if state-based lighting
                    match state.lighting_mode {
                        LightingMode::StateBased => vec![Command::SetPattern {
                            pattern: LedPattern::Pulse,
                            colors: vec![RgbColor::new(20, 20, 20)],
                        }],
                        _ => vec![], // Don't change ambient lighting
                    }
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        }

        // ... handle all other events
    }
}
```

**LLM Service** (`llm_service.rs`):
```rust
pub struct LlmService {
    client: OllamaClient,
    model: String,
}

impl LlmService {
    pub async fn new(config: LlmConfig) -> Self {
        // Initialize Ollama client
    }

    pub async fn generate_response(
        &mut self,
        user_input: &str,
        context: &ConversationContext,
        state: &BotState,
    ) -> String {
        // Build prompt with context + memories + system state
        // Call Ollama API
        // Return response
    }
}
```

**Memory Service** (`memory_service.rs`):
```rust
pub struct MemoryService {
    short_term: VecDeque<Exchange>, // Last 10 exchanges
    session_storage: PathBuf,       // Today's conversation
    long_term_storage: PathBuf,     // Persistent facts
}

impl MemoryService {
    pub async fn store_exchange(&mut self, user: &str, bot: &str) {
        // Store in short-term
        // Write to session file
        // Extract facts for long-term storage
    }

    pub async fn get_relevant_context(&self, query: &str) -> ConversationContext {
        // Retrieve relevant memories
        // Build context object
    }
}
```

---

### audio_pipeline - STT/TTS

**Purpose**: Audio processing pipeline

**Wake Word Detection** (`wake_word.rs`):
```rust
use vosk::{Model, Recognizer};

pub struct WakeWordDetector {
    recognizer: Recognizer,
    wake_phrase: String,
}

impl WakeWordDetector {
    pub fn new(model_path: &str, wake_phrase: &str) -> Result<Self> {
        let model = Model::new(model_path)?;
        let recognizer = Recognizer::new(&model, 16000.0)?;
        Ok(Self {
            recognizer,
            wake_phrase: wake_phrase.to_lowercase(),
        })
    }

    pub async fn listen(&mut self) -> Result<bool> {
        // Continuously monitor audio stream
        // Return true when wake phrase detected in transcription
    }
}
```

**Speech-to-Text** (`stt.rs`):
```rust
use vosk::{Model, Recognizer};

pub struct SpeechRecognizer {
    recognizer: Recognizer,
}

impl SpeechRecognizer {
    pub async fn transcribe(&self, audio: &[i16]) -> Result<String> {
        // Process audio through Vosk
        // Return transcribed text
    }
}
```

**Text-to-Speech** (`tts.rs`):
```rust
pub struct PiperTts {
    // Piper engine
}

impl PiperTts {
    pub async fn synthesize(&self, text: &str) -> Result<Vec<u8>> {
        // Generate audio from text using Piper
        // Return audio bytes
    }
}
```

---

### vision_pipeline - Camera ML

**Purpose**: Process camera frames for environmental awareness

**Human Detection** (`human_detection.rs`):
```rust
pub struct HumanDetector {
    // ML model for person detection
}

impl HumanDetector {
    pub async fn detect_humans(&self, frame: &Frame) -> Vec<Detection> {
        // Run inference on frame
        // Return bounding boxes + confidence
    }
}
```

**Integration**: Lightweight model running on-device (MobileNet SSD or similar)

---

### runner - Orchestration

**Purpose**: Bootstrap system and spawn all component tasks

**Main Structure** (`main.rs`):
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = load_config("config/config.yaml")?;

    // Create channels
    let (event_tx, event_rx) = mpsc::channel::<Event>(64);
    let (led_tx, led_rx) = mpsc::channel::<Command>(32);
    let (speaker_tx, speaker_rx) = mpsc::channel::<Command>(32);
    let (status_tx, status_rx) = mpsc::channel::<Command>(32);
    let (lcd_tx, lcd_rx) = mpsc::channel::<Command>(32);
    let (shutdown_tx, _) = broadcast::channel::<()>(16);

    // Spawn sensor tasks
    let pir_handle = tokio::spawn(pir_sensor_task(
        event_tx.clone(),
        shutdown_tx.subscribe(),
        config.gpio.pir_pin,
    ));

    let audio_handle = tokio::spawn(audio_sensor_task(
        event_tx.clone(),
        shutdown_tx.subscribe(),
        config.audio.clone(),
    ));

    // ... spawn all other sensor tasks

    // Spawn actuator tasks
    let rgb_handle = tokio::spawn(rgb_led_actuator_task(
        led_rx,
        shutdown_tx.subscribe(),
        config.gpio.rgb_pins,
    ));

    let speaker_handle = tokio::spawn(speaker_actuator_task(
        speaker_rx,
        shutdown_tx.subscribe(),
        config.audio.clone(),
    ));

    // ... spawn all other actuator tasks

    // Spawn AI controller
    let controller_handle = tokio::spawn(run_controller(
        event_rx,
        led_tx,
        speaker_tx,
        status_tx,
        lcd_tx,
        shutdown_tx.subscribe(),
        config.bot,
    ));

    // Spawn health monitor
    let health_handle = tokio::spawn(health_monitor_task(
        event_tx.clone(),
        shutdown_tx.subscribe(),
    ));

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("Shutdown signal received");

    // Broadcast shutdown
    shutdown_tx.send(())?;

    // Wait for all tasks to complete
    tokio::try_join!(
        pir_handle,
        audio_handle,
        rgb_handle,
        speaker_handle,
        controller_handle,
        health_handle,
    )?;

    println!("Pi Bot shut down gracefully");
    Ok(())
}
```

**Task Modules**: Individual task implementations in separate files

---

## Rust/Python Integration

### Python Subprocess Pattern

For components requiring Python libraries (DHT11, camera):

```rust
use tokio::process::Command;

pub async fn read_dht11() -> Result<(f32, f32)> {
    let output = Command::new("python3")
        .arg("bot_utils/dht11.py")
        .output()
        .await?;

    let stdout = String::from_utf8(output.stdout)?;
    let values: Vec<&str> = stdout.trim().split(',').collect();

    let temp = values[0].parse()?;
    let humidity = values[1].parse()?;

    Ok((temp, humidity))
}
```

**Python Script Example** (`bot_utils/dht11.py`):
```python
#!/usr/bin/env python3
import sys
from gpiozero import DHT11
from time import sleep

sensor = DHT11(pin=4)
temp = sensor.temperature
humidity = sensor.humidity

print(f"{temp},{humidity}")
```

**Advantages**:
- Leverage Python ecosystem for hardware that lacks Rust support
- Process isolation (Python crash doesn't kill Rust system)
- Easy to test Python scripts independently

**Package Management**: Use `uv` for Python dependencies

---

## Testing Strategy

### 1. Component-Level Tests

**Hardware Test Binaries**: Located in each crate's `src/bin/`

**Example**: `sensors/src/bin/mic-test.rs`
```rust
// Simple microphone test
use sensors::AudioController;

#[tokio::main]
async fn main() {
    println!("Testing microphone...");

    let mut mic = AudioController::new().expect("Failed to init mic");

    println!("Listening for wake word 'hey-bot'...");
    loop {
        if let Some(detected) = mic.check_for_wake_word().await {
            println!("✓ Wake word detected!");
            break;
        }
    }

    println!("Now speak a sentence...");
    if let Some(text) = mic.capture_speech().await {
        println!("✓ Transcribed: {}", text);
    }
}
```

**Run Test**: `cargo run --bin mic-test`

### 2. Integration Tests

Test event flow without hardware:

```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_wake_word_to_response_flow() {
    let (event_tx, event_rx) = mpsc::channel(32);
    let (led_tx, mut led_rx) = mpsc::channel(32);
    let (speaker_tx, mut speaker_rx) = mpsc::channel(32);

    // Spawn controller
    tokio::spawn(run_controller(event_rx, led_tx, speaker_tx, ...));

    // Simulate wake word detection
    event_tx.send(Event::WakeWordDetected).await.unwrap();

    // Verify LED changes to listening state
    let cmd = led_rx.recv().await.unwrap();
    assert!(matches!(cmd, Command::SetPattern { pattern: LedPattern::Breathing, .. }));

    // Simulate speech capture
    event_tx.send(Event::SpeechCaptured("Hello bot".to_string())).await.unwrap();

    // Verify speaker receives command
    let cmd = speaker_rx.recv().await.unwrap();
    assert!(matches!(cmd, Command::Speak { .. }));
}
```

### 3. End-to-End Tests

Manual testing with hardware wired up:
1. Test each component individually with test binaries
2. Run full system with `cargo run --bin runner`
3. Verify event flow through console logging
4. Test conversation scenarios from FEATURES.md

---

## Error Handling & Fault Tolerance

### Component Supervisor Pattern

```rust
async fn supervised_sensor_task<F, Fut>(
    name: &str,
    event_tx: mpsc::Sender<Event>,
    shutdown_rx: broadcast::Receiver<()>,
    task_fn: F,
) where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<()>>,
{
    loop {
        tokio::select! {
            result = task_fn() => {
                match result {
                    Ok(_) => {
                        log::info!("{} task completed normally", name);
                        break;
                    }
                    Err(e) => {
                        log::error!("{} task failed: {:?}", name, e);

                        // Report health status
                        let _ = event_tx.send(Event::ComponentHealth {
                            component: name.to_string(),
                            healthy: false,
                        }).await;

                        // Wait before restart
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        log::info!("Restarting {} task", name);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                log::info!("{} task shutting down", name);
                break;
            }
        }
    }
}
```

### Health Monitoring

```rust
async fn health_monitor_task(
    event_tx: mpsc::Sender<Event>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let mut component_health = HashMap::new();

    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                if let Event::ComponentHealth { component, healthy } = event {
                    component_health.insert(component, healthy);

                    // Determine overall system health
                    let critical_ok = ["llm", "audio", "rgb_led"]
                        .iter()
                        .all(|c| component_health.get(*c).unwrap_or(&false));

                    if !critical_ok {
                        log::error!("Critical component failure detected!");
                        // Flash status LEDs red
                    }
                }
            }
            _ = shutdown_rx.recv() => break,
        }
    }
}
```

---

## Dependencies

### Rust Crates

**Core**:
- `tokio` - Async runtime
- `serde` / `serde_yaml` - Configuration
- `log` / `env_logger` - Logging
- `anyhow` / `thiserror` - Error handling

**Hardware**:
- `rppal` - GPIO, PWM, I2C
- `opencv` - Camera processing (optional, can use picamera2 via Python)

**Audio**:
- `vosk` - Wake word detection + speech-to-text (offline)
- `cpal` - Audio I/O
- `tokio-process` - Piper TTS subprocess

**AI**:
- `reqwest` - Ollama API client
- `serde_json` - JSON memory storage

### Python Packages

Managed with `uv`:
- `gpiozero` - DHT11 sensor
- `lgpio` - Low-level GPIO for DHT11
- `picamera2` - Camera interface (if not using Rust)

---

## Build & Deployment

### Development Build

```bash
# Build entire workspace
cargo build --workspace

# Run main system
cargo run --bin runner

# Test individual components
cargo run --bin mic-test
cargo run --bin rgb-test
cargo run --bin camera-test

# Check for errors
cargo check --workspace

# Run tests
cargo test --workspace
```

### Release Build

```bash
# Optimized build for Pi
cargo build --release --workspace

# Install to system
sudo cp target/release/runner /usr/local/bin/pi-bot
```

### Systemd Service

```ini
# /etc/systemd/system/pi-bot.service
[Unit]
Description=Pi Bot Companion
After=network.target

[Service]
Type=simple
User=pi
WorkingDirectory=/home/pi/pi-bot
ExecStart=/usr/local/bin/pi-bot
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

---

## Development Workflow

### Phase 1: Component Bring-Up

1. Start with critical path: Audio → LLM → Speaker
2. Test each component individually with test binaries
3. Verify wiring and GPIO configuration
4. Build minimal conversation flow

### Phase 2: Integration

1. Wire up event channels in runner
2. Implement basic controller logic
3. Test end-to-end wake word → response flow
4. Add LED feedback

### Phase 3: Enhancement

1. Add remaining sensors (camera, DHT11)
2. Implement passive observation mode
3. Add memory system
4. Refine personality and patterns

---

## Success Metrics

### Testability Checklist

- [ ] Each hardware component has standalone test binary
- [ ] Test binaries use only print statements (no complex logic)
- [ ] Can test components before full system integration
- [ ] Component failures don't crash entire system

### Architecture Checklist

- [ ] Sensors only emit events
- [ ] Actuators only consume commands
- [ ] All business logic in AI controller
- [ ] No shared mutable state
- [ ] Clean shutdown handling
- [ ] Component independence verified (can unplug sensor without crash)

### Performance Targets

- Wake word detection latency: <200ms
- STT latency: <2s for 5-second speech
- LLM response generation: <3s
- TTS latency: <1s
- End-to-end (wake word → spoken response): <6s

---

## Next Steps

1. Review this architecture with user
2. Set up initial workspace structure
3. Implement test binaries for critical components
4. Build minimal conversation flow (audio → LLM → speaker)
5. Iterate on remaining features

Let's build a bot that feels alive! 🤖
