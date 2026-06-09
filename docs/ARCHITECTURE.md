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
│  │ 1. Initialise channels                                     │ │
│  │ 2. Spawn AI controller + command distributor               │ │
│  │ 3. Spawn actuators → wait for ComponentReady × 5          │ │
│  │ 4. Spawn sensors   → wait for ComponentReady × 2          │ │
│  │ 5. Wait for Ctrl+C (shutdown)                              │ │
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

## Startup Sequence

The runner follows a strict startup order to ensure LED loading indicators
are correct and that the AI controller **owns all LED state** — the runner
never sends commands directly to actuators.

```
1. Channels initialised
        │
2. AI controller + command distributor spawned
   (Controller begins loading LLM/memory in its task)
        │
3. Actuators spawned (RGB, Green, Red LEDs, Speaker, LCD)
   Red + RGB LEDs default to red-breathing pattern (loading state)
        │
4. Each actuator sends ComponentReady → runner forwards to event bus
   Controller receives ComponentReady × 5 → sends SetRedLeds(Breathing)
        │
5. Sensors spawned (PIR, Audio/Vosk)
        │
6. Each sensor sends ComponentReady → runner forwards to event bus
   Controller receives ComponentReady × 7 (all) → sends:
     SetGreenLeds(Solid)   ← system ready
     SetRedLeds(Off)
     RGB LED → Ready pattern
        │
7. Main loop: controller processes events until Ctrl+C
        │
8. Shutdown signal broadcast → all tasks stop → LEDs off
```

**Key invariant**: The runner only forwards startup signals as events. It
never sends commands to actuators directly. All LED state is owned by the
controller.

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
// events.rs — what sensors emit
pub enum Event {
    // Presence
    PresenceDetected,
    NoPresenceSince(Duration),

    // Audio
    WakeWordDetected,
    SpeechCaptureStarted,
    SpeechCaptured(String),
    SpeechPlaybackStarted,
    SpeechComplete,
    AmbientNoiseLevel(u8),

    // System
    ComponentReady { component: String },   // emitted once per component on startup
    ComponentHealth { component: String, healthy: bool },
    SystemReady,                            // legacy; superseded by ComponentReady

    // User actions (DND)
    UserRequestedDND,
    UserRequestedWakeUp,
}

// commands.rs — what the controller sends to actuators
pub enum Command {
    // RGB LED
    SetColor(RgbColor),
    SetPattern { pattern: LedPattern, colors: Vec<RgbColor> },
    LedOff,

    // Status LEDs
    SetGreenLeds(StatusLedPattern),   // Solid | Breathing | Flashing | Off
    SetRedLeds(StatusLedPattern),

    // LCD
    DisplayText { line1: String, line2: String, duration_ms: Option<u64> },
    ClearDisplay,
    SetBacklight { on: bool },

    // Audio
    Speak { text: String },
    StopSpeaking,
    StartListening,
    StopListening,

    // State
    LockBot,
    UnlockBot,
    EnterConversationState(ConversationState),
    SetLightingMode(LightingMode),
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
    cmd_tx: mpsc::Sender<Command>,
    mut shutdown_rx: broadcast::Receiver<()>,
    config: SystemConfig,
) -> Result<()> {
    let mut state = ControllerState::new(config).await?;

    // Enter loading state immediately (red LEDs breathing)
    send_loading_commands(&cmd_tx).await?;

    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => handle_event(event, &mut state, &cmd_tx).await?,
            _ = timeout_interval.tick() => check_conversation_timeout(&mut state, &cmd_tx).await?,
            _ = shutdown_rx.recv() => break,
        }
    }
}
```

**Startup tracking**: The controller maintains a `StartupTracker` that records
which components have reported `ComponentReady`. When all 7 components are ready
(5 actuators + 2 sensors), the controller transitions to the `Ready` conversation
state and sends the "green LEDs solid, red LEDs off" commands.
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
    short_term: VecDeque<Exchange>, // Last N exchanges (configurable)
    session_storage: PathBuf,       // Today's conversation (JSON)
    facts: Vec<Fact>,               // Long-term semantic facts
    embedder: EmbeddingService,     // ONNX all-MiniLM-L6-v2
}
```

See the **Memory System** section below for full details.

---

### Memory System

Pi Bot uses a three-tier memory architecture that provides both immediate
conversational context and persistent long-term recall across sessions.

```
┌──────────────────────────────────────────────────────────────────┐
│                         MemoryService                            │
├──────────────────────────────────────────────────────────────────┤
│  Tier 1: Short-Term          Tier 2: Session       Tier 3: Facts │
│  (RAM, ephemeral)            (disk, daily)         (disk, durable)│
│  ┌───────────────┐           ┌─────────────┐       ┌──────────┐  │
│  │ Last N        │ persist → │ YYYY-MM-DD  │       │facts.json│  │
│  │ exchanges     │           │ .json       │       │+ vectors │  │
│  │ (configurable)│           │             │       │          │  │
│  └───────────────┘           └─────────────┘       └──────────┘  │
│         │                           │                    ▲        │
│         │ on startup:               │ cross-day seed     │        │
│         │ load recent←──────────────┘                   │extract │
│         │ from prev session                              │(LLM)   │
│         │                                               │        │
│         └─────── semantic search ────────────────────────        │
│                   (before LLM call)                              │
└──────────────────────────────────────────────────────────────────┘
```

#### Tier 1 — Short-Term (RAM)

Holds the last `max_short_term` exchanges (default: 10) in memory as a
sliding window. Passed directly to the LLM as conversation history on each
turn. Cleared on restart; session files persist it across restarts.

**Cross-day continuity**: when the bot starts a brand-new session (no
exchanges yet today), it seeds short-term memory with the last 3 exchanges
from the most recent previous session, so conversations don't feel cold.

#### Tier 2 — Session Storage (disk)

One JSON file per calendar day at `data/sessions/YYYY-MM-DD.json`.
All exchanges are appended after every turn, auto-saving the full
transcript. Older sessions are never deleted automatically.

#### Tier 3 — Long-Term Semantic Memory (disk)

Stores durable *facts* extracted from conversations as structured records
with dense vector embeddings, enabling semantic (meaning-based) search.

**Embedding model**: `all-MiniLM-L6-v2` (ONNX, 80 MB, 384-dimensional vectors)
located at `models/embeddings/all-MiniLM-L6-v2.onnx`.

**Fact schema**:
```rust
pub struct Fact {
    pub id: String,
    pub text: String,              // e.g. "User prefers tea in the morning"
    pub embedding: Vec<f32>,       // 384-dim normalised vector
    pub timestamp: DateTime<Utc>,
    pub source: FactSource,        // UserTold | Conversation | Observation
    pub category: Option<String>,
    pub relevance_count: u32,      // how often retrieved (for future ranking)
    pub confidence: f32,
}
```

**Storage**: `data/memory/facts.json` (plain JSON, no dependency on SQLite).

#### Memory Retrieval (before each LLM call)

```
User says: "What should I have for lunch?"
                    │
                    ▼
       EmbeddingService.embed(query)      ← ~50ms
                    │
                    ▼
  cosine_similarity(query_vec, all_facts)
                    │
                    ▼
  top-K facts with similarity ≥ threshold  (default: K=5, threshold=0.7)
                    │
                    ▼
  Injected as system message:
  "Relevant facts about the user:
   - User dislikes spicy food [confidence: 82%]
   - User usually eats lunch at 1pm [confidence: 74%]"
                    │
                    ▼
            LLM generates response
```

#### Fact Extraction (after each exchange)

Runs *after* the bot finishes speaking to avoid adding latency. Uses a
deterministic, low-temperature LLM call with explicit rules to preserve
the user's exact wording while converting first-person statements to
third-person facts.

```
Bot finishes speaking (SpeechComplete event)
                    │
         pending_extraction set? yes
                    │
                    ▼
         LED → Learning (purple pulse)
                    │
                    ▼
  LLM: extract_facts(user_msg, assistant_response)
    temperature: 0.0 (deterministic)
    context: 512 tokens

    Extraction rules enforced by prompt:
    1. Convert "I [verb] X" → "User [verb] X"
    2. Preserve X EXACTLY as stated (no interpretation/generalization)
    3. Do NOT add assumptions or rephrase content
    4. Extract only personal facts (preferences, habits, work, relationships)

    Examples shown in prompt:
    ✓ "I like coffee" → ["User likes coffee"]
    ✗ "I like spicy ramen" → ["User is spicy"]  # WRONG (misinterpreted)
    ✗ "I like craft beer" → ["User likes beer"]  # GENERALIZED (lost detail)
    ✓ "I like craft beer" → ["User likes craft beer"]  # CORRECT (exact)
                    │
                    ▼
  For each extracted fact:
    embed → cosine check (similarity ≥ 0.9 → deduplicate)
    store in fact_database → save facts.json
                    │
                    ▼
         LED → Ready (green breathing)
```

**Why this approach?**
Previous versions used a simple extraction prompt which caused the LLM to
interpret and generalize user statements. The new prompt includes explicit
rules and contrasting examples to enforce literal preservation.

#### Explicit Memory Commands (AI-Detected)

Users can manage the fact database via natural language, detected using a
fast LLM call (temperature 0.0, 256 tokens) before the main conversation.

```
User says: "Can you remember that I like coffee?"
                    │
                    ▼
  detect_memory_command(text, llm) → async LLM call:
    - Analyzes user intent
    - Returns JSON: {"intent": "remember"|"forget"|"none", "content": "fact"}
    - Catches natural variations:
        * "remember that..."
        * "don't forget..."
        * "keep in mind that..."
        * "you should know..."
                    │
                    ▼
  If intent == "remember":
    store fact immediately (before main LLM call)
    → LLM responds naturally: "I'll remember that!"

  If intent == "forget":
    semantic search → remove closest matching fact
    → LLM responds: "I've forgotten that."
```

**Why AI detection?**
Hardcoded phrase matching (e.g., `if text.starts_with("remember that")`)
is rigid and misses natural variations. The AI-based approach:
- Understands implicit phrasing ("don't forget I'm vegan")
- Works with any natural language variant
- Runs before the main LLM call so facts are stored transparently
- Fails silently on errors (never blocks conversation flow)

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

**Purpose**: Bootstrap system and spawn all component tasks in startup order

**Startup order** (strictly sequential to guarantee correct LED states):

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config("config/config.yaml")?;

    // 1. Create channels
    let (event_tx, event_rx) = mpsc::channel::<Event>(64);
    let (startup_tx, mut startup_rx) = mpsc::channel::<(String, bool)>(16);
    let (controller_cmd_tx, controller_cmd_rx) = mpsc::channel::<Command>(32);
    // ... per-actuator command channels ...
    let (shutdown_tx, _) = broadcast::channel::<()>(16);

    // 2. Spawn AI controller + command distributor first
    tokio::spawn(run_controller(event_rx, controller_cmd_tx, shutdown_rx, config.clone()));
    tokio::spawn(run_command_distributor(controller_cmd_rx, ...));

    // 3. Spawn actuators (each sends startup signal when hardware is ready)
    tokio::spawn(run_rgb_led_actuator(&config, rgb_rx, startup_tx.clone(), shutdown_rx));
    tokio::spawn(run_green_led_actuator(...));
    tokio::spawn(run_red_led_actuator(...));
    tokio::spawn(run_speaker_actuator(...));
    tokio::spawn(run_lcd_actuator(...));

    // 4. Wait for 5 actuator signals; forward each as ComponentReady to controller
    for _ in 0..ACTUATOR_COUNT {
        let (name, _) = startup_rx.recv().await?;
        event_tx.send(Event::ComponentReady { component: name }).await?;
    }

    // 5. Spawn sensors (after all actuators are ready)
    tokio::spawn(run_pir_sensor(&config, event_tx.clone(), startup_tx.clone(), shutdown_rx));
    tokio::spawn(run_audio_sensor(&config, event_tx.clone(), audio_cmd_rx, shutdown_rx, startup_tx));

    // 6. Wait for 2 sensor signals; forward each as ComponentReady to controller
    for _ in 0..SENSOR_COUNT {
        let (name, _) = startup_rx.recv().await?;
        event_tx.send(Event::ComponentReady { component: name }).await?;
    }
    // Controller receives all 7 ComponentReady events → transitions to Ready state

    // 7. Wait for Ctrl+C and broadcast shutdown
    tokio::signal::ctrl_c().await?;
    shutdown_tx.send(())?;
    // ... join all tasks
}
```

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
