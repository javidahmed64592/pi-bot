# Pi Bot - Feature Documentation

## Overview

Pi Bot is an AI-driven companion that lives on your desk and interacts with you through conversation, lighting, and environmental awareness.
Unlike a passive assistant, Pi Bot has personality, curiosity, and the ability to initiate interactions based on what it observes.

The bot operates in multiple behavioral modes and uses visual feedback (RGB LED patterns) to communicate its state, creating a dynamic and emotionally engaging presence.

## Design Philosophy

### Personality-First Design

Pi Bot is designed to feel **alive** rather than transactional. Key principles:

- **Proactive engagement**: The bot can decide to start conversations based on environmental observations
- **Emotional expression**: LED colors and patterns reflect the bot's internal state
- **Persistent memory**: Learns about you over time and adapts its personality
- **Contextual awareness**: Understands when to be quiet, helpful, or playful
- **Natural interaction**: Uses wake phrases but can also interrupt you (politely) when appropriate

### Behavioral Modes

The bot separates **conversation behavior** from **lighting presentation** for flexibility:

#### Conversation States

| State | Description | When Bot Initiates Conversation | Trigger |
|-------|-------------|----------------------------------|---------|
| **Ready** | Default monitoring state, ready for interaction | Yes (via Passive Observation) | System start, conversation ends, user returns |
| **Observing** | Noticed something interesting, deciding whether to speak | Maybe (evaluating context) | Random intervals, interesting sensor data |
| **Active** | In conversation (Listening → Thinking → Speaking) | No (already in conversation) | Wake phrase detected or bot decided to speak |
| **Silent** | Do Not Disturb - won't initiate, minimal responses | No | User request ("I'm in a meeting") |

#### Lighting Modes

| Mode | Description | Example Patterns | When |
|------|-------------|------------------|------|
| **State-based** | LED reflects conversation state | Orange (listening), Green (speaking), Blue (thinking) | Default, during active conversation |
| **Ambient** | Decorative pattern, independent of state | Gradient, rainbow, slow color cycle, warm colors | User requests ambient lighting |
| **Minimal** | Dim or off | Very dim white or off | Silent mode, user preference |

**Key Insight**: Conversation state and lighting mode are **independent**. You can have ambient lighting while the bot is Ready (can talk) or Silent (won't talk unless woken).

#### Active Conversation Sub-States

When in **Active** conversation state, the bot cycles through:

| Sub-State | Description | Default LED (State-based mode) |
|-----------|-------------|-------------------------------|
| **Listening** | Actively recording and processing speech | Orange breathing animation |
| **Thinking** | Processing input through LLM | Blue pulsing |
| **Speaking** | Generating audio output | Solid green |
| **Learning** | Storing important memory | Purple pulsing |

---

### Common State Combinations

For quick reference, here are the most common combinations:

| Use Case | Conversation State | Lighting Mode | Bot Behavior |
|----------|-------------------|---------------|--------------|
| **Normal operation** | Ready | State-based | Default - bot can talk, LED shows state |
| **Coding with ambiance** | Ready | Ambient | Pretty lights, bot can still check in on you |
| **Bedtime lighting** | Silent | Ambient | Pretty lights, bot won't talk unless woken |
| **Meeting mode** | Silent | Minimal | Quiet and unobtrusive, only responds if needed |
| **Active conversation** | Active | State-based | Talking with visual feedback (orange/blue/green) |
| **Ambient conversation** | Active | Ambient | Talking but lights stay ambient (user preference) |

**Default on startup**: Ready + State-based

**User can configure**: Whether ambient lighting persists during conversations or switches to state-based

## Component Integration

### Sensors

#### Pi Camera Module v3 (Vision)
**Purpose**: Visual observation of the environment

**Enabled Behaviors**:
- Detect human presence in field of view
- Recognize if user is sitting at desk for extended periods
- Identify objects or changes in environment (future: gesture recognition)
- Capture context for conversation (e.g., "I see you're working on something")

**Integration**:
- Emits `VisionEvent::HumanDetected`, `VisionEvent::DeskOccupied`, `VisionEvent::ObjectChange` (TODO: These are TBD and will evolve as we implement vision features)
- Processed through lightweight ML model on-device
- Used by AI controller to decide if it should initiate conversation

**Failure Behavior**: Bot continues operating without vision, relying on audio and other sensors

---

#### USB Microphone (Hearing)
**Purpose**: Capture audio for wake word detection and speech-to-text

**Enabled Behaviors**:
- Wake phrase detection ("Hey Bot")
- Speech-to-text conversion for user commands/conversation
- Ambient noise detection (can tell if music is playing)

**Integration**:
- Vosk library continuously monitors for wake phrase using keyword spotting
- When triggered, switches to full Vosk recognition for STT
- Emits `AudioEvent::WakeWordDetected`, `AudioEvent::SpeechCaptured(text)`

**Failure Behavior**: Bot loses voice interaction but can still operate visual observation mode

---

#### PIR Motion Sensor (Presence)
**Purpose**: Detect human movement/presence in room

**Enabled Behaviors**:
- Determine if user is in the room
- Trigger passive observation mode after periods of inactivity
- Used as context for AI decisions (don't talk if no one is present)

**Integration**:
- Emits `MotionEvent::PresenceDetected`, `MotionEvent::NoPresenceSince(duration)`
- Low-power continuous monitoring
- Combined with camera data for robust presence detection

**Failure Behavior**: Bot relies on camera and audio for presence detection

---

#### DHT11 (Temperature & Humidity)
**Purpose**: Monitor environmental conditions

**Enabled Behaviors**:
- Inform user about uncomfortable conditions
- Provide context for conversation ("It's pretty humid today")
- Historical tracking for memory system

**Integration**:
- Emits `EnvironmentEvent::Reading { temp, humidity }`
- Uses Python subprocess
- Low-frequency polling (every 2-5 minutes)

**Failure Behavior**: Bot loses environmental awareness but continues operating

---

### Actuators

#### RGB LED (Primary Expression)
**Purpose**: Main visual communication channel for bot's state

**Color States**:
- **Green**: Ready state (breathing), Speaking (solid)
- **Orange**: Listening, processing (breathing)
- **Blue**: Observing state (breathing), Thinking (pulsing)
- **Red**: Silent/DND state (breathing)
- **Purple**: Learning/remembering something (pulsing)
- **Warm colors (red-orange-yellow)**: Ambient comfort lighting
- **Cool colors (blue-cyan)**: Focused work lighting
- **Rainbow**: Celebration or music mode

**Pattern Types**:
- **Solid**: Active state (speaking, displaying)
- **Breathing**: Passive state (listening, waiting)
- **Pulse**: Thinking, processing
- **Gradient**: Ambient mode, decorative
- **Rainbow cycle**: Music mode, celebration
- **Fast pulse**: Urgent attention needed

**Integration**:
- Receives `LedCommand::SetColor(color)`, `LedCommand::SetPattern(pattern, color)`
- Hardware PWM control for smooth transitions
- Pattern executor runs independently with smooth interpolation

**Failure Behavior**: Bot loses visual expression but continues audio interaction

---

#### Speaker + Amplifier (Voice)
**Purpose**: Audio output for bot's speech

**Enabled Behaviors**:
- Text-to-speech output using Piper
- Natural conversation responses
- Playful interjections ("Get up, lazy!")

**Integration**:
- Receives `AudioCommand::Speak(text, emotion)` (TODO: Emotion parameter can be linked to RGB LED colour for enhanced expression)
- Piper TTS with configurable voice model
- Async playback doesn't block other operations

**Failure Behavior**: Bot becomes mute but can still use LED patterns to communicate states

---

#### Status LEDs - Green (2x) (Active State Indicators)
**Purpose**: Indicate bot active states independent of main RGB LED

**Patterns**:
- **Solid Green**: Bot is active and ready (Ready state, awaiting interaction)
- **Breathing Green**: Bot is processing or listening (Active.Listening, Active.Thinking sub-states)
- **Off**: Bot is idle or in error state (see Red LEDs)

**Integration**:
- Receives `StatusCommand::SetGreenLeds(pattern)` with patterns: Solid, Breathing, Off
- Independent control from RGB LED for parallel visual feedback
- Always-on during normal operations to indicate bot availability

**Use Cases**:
- User can quickly glance to see if bot is ready to interact (green = ready)
- Breathing animation provides visual feedback during processing without affecting RGB mood lighting

**Failure Behavior**: Bot loses status indication but continues normal operation

---

#### Status LEDs - Red (2x) (Idle/Error State Indicators)
**Purpose**: Indicate bot idle/error states and system health

**Patterns**:
- **Breathing Red**: Bot is idle or in Do Not Disturb mode (Silent state)
- **Flashing Red**: System error detected (component failure, critical issue)
- **Off**: Bot is active (see Green LEDs)

**Integration**:
- Receives `StatusCommand::SetRedLeds(pattern)` with patterns: Breathing, Flashing, Off
- Independent control for clear visual separation from active states
- Used for fault diagnosis and user awareness of bot availability

**Use Cases**:
- **DND Mode**: Bot enters Silent state → Red LEDs breathe → Clear visual indicator not to disturb
- **System Error**: Component fails → Red LEDs flash → User knows to check system status
- **Normal Operation**: Red LEDs off → Green LEDs on → Bot ready for interaction

**Failure Behavior**: N/A (critical component for fault diagnosis)

---

#### LCD Display (Text Output)
**Purpose**: Display text information and system state

**Display Modes**:
- Line 1: Current bot state or last spoken phrase
- Line 2: Environmental data (temp, humidity, time)
- Scrolling for long messages

**Integration**:
- Receives `DisplayCommand::ShowText(line1, line2)`
- I2C communication via PCF8574 backpack
- Non-critical, decorative enhancement

**Failure Behavior**: Bot continues without text display

---

## AI Behavior System

### Ready State (Default)

**Trigger**: System start, conversation ends, user returns to room

**Behavior**:
- Monitoring all sensors continuously
- Ready to respond to wake phrase immediately
- May enter Observing state based on random intervals or sensor triggers
- Minimal CPU usage
- Lighting: State-based (minimal/off) or Ambient (if user configured)

**Transitions**:
- → **Observing**: Random timer (5-15 min) or interesting sensor event
- → **Active**: Wake phrase detected
- → **Silent**: User requests Do Not Disturb

---

### Observing State

**Trigger**: Random intervals (5-15 minutes) or interesting sensor events

**Decision Logic**:
1. Camera detects user at desk (or PIR shows presence)
2. Check if user appears busy (rapid movement, typing detected)
3. Evaluate context (time of day, conversation state, current activity)
4. Roll dice: 20% chance to initiate conversation (configurable)
5. If yes: generate observation-based opener and transition to **Active**
6. If no: return to **Ready**

**Example Triggers**:
- "You've been sitting for 2 hours, want to stretch?"
- "The temperature dropped 5 degrees, you feeling cold?"
- "I see you moved something on your desk, what is it?"
- "Hey, you look stressed, want to talk about it?"

**LED Pattern**: Subtle dim pulse (if state-based) or continues ambient pattern

**Transitions**:
- → **Active**: Bot decides to speak
- → **Ready**: Bot decides not to speak

---

### Active State (Conversation)

**Trigger**: User says wake phrase OR bot initiates conversation from Observing

**Flow**:
1. **Listening**: Capture speech (orange breathing if state-based LED)
2. **Thinking**: Process through LLM (blue pulsing if state-based LED)
3. **Learning**: Store memory if important (brief purple flash)
4. **Speaking**: Play TTS response (solid green if state-based LED)
5. Return to **Listening** and wait for user response
6. After 10s of silence → Return to **Ready**

**Context Included in LLM**:
- Recent conversation history (last 10 exchanges)
- Environmental data (temp, humidity, time of day)
- Presence data (how long user has been at desk)
- Persistent memory (facts learned about user)
- Current conversation state and emotional tone

**Lighting**:
- **State-based mode**: Orange (listening) → Blue (thinking) → Green (speaking)
- **Ambient mode**: Pattern continues, no state indication (unless user prefers hybrid)

**Transitions**:
- → **Listening**: User is speaking
- → **Ready**: 10 seconds of silence
- → **Silent**: User requests Do Not Disturb mid-conversation

---

### Silent State (Do Not Disturb)

**Trigger**: User explicitly requests quiet time

**Behavior**:
- No conversation initiation (won't enter Observing)
- Still monitors sensors passively
- Still responds to wake phrase, but with minimal responses
- Concise answers only, no follow-up questions
- Automatically expires after set duration (default: 1 hour) or user ends it

**Lighting**: Minimal (dim or off) by default, or Ambient if user specified

**Example**:
```
User: "Hey Bot, I'm in a meeting, be quiet for an hour"
Bot: "Got it, I'll be quiet. Talk to you later."
→ Silent state + Minimal lighting for 1 hour
```

**Transitions**:
- → **Ready**: Timer expires or user says "Hey Bot, meeting's over"
- → **Active**: User says wake phrase (only for brief, essential responses)

---

### Lighting Mode Transitions

**Switching to Ambient Mode**:
```
User: "Hey Bot, give me some warm ambient lighting"
Bot: "Setting up warm ambient lighting for you"
→ Lighting mode changes to Ambient (warm gradient)
→ Conversation state stays Ready (bot can still talk)
```

**Ambient + Silent (Bedtime)**:
```
User: "Hey Bot, light up the room but don't talk unless I need you"
Bot: "Setting up ambient lighting. I'll stay quiet unless you need me."
→ Lighting mode: Ambient (warm gradient)
→ Conversation state: Silent (won't initiate)
```

**Returning to State-based**:
- Ambient lighting automatically ends when:
  - User requests state-based mode explicitly
  - Bot is turned off/restarted (reverts to default)
  - User starts conversation (optional: can stay in ambient if preferred)
- User can configure whether ambient persists across conversations

---

## Interaction Scenarios

### Scenario 1: Bot-Initiated Check-In

**Context**: User sitting at desk for 2 hours, camera detects minimal movement

1. **Bot observes**: PIR + camera show continuous presence at desk
2. **Bot decides**: "User might need break reminder"
3. **LED changes**: Dim green breathing (preparing to speak)
4. **Bot speaks**: "Hey, you've been sitting for a while, want to get up and stretch?"
5. **User responds**: "Yeah, good idea"
6. **LED changes**: Orange breathing (listening)
7. **Bot speaks**: "Great! I'll put on some energizing lights for you" → Rainbow pulse
8. **Returns to idle** after user returns

---

### Scenario 2: Nighttime Anxiety Lighting

**Context**: Late night, room is dark, user feeling anxious

1. **User says**: "Hey Bot, it's too dark and I'm anxious, can you light up the room?"
2. **LED changes**: Orange breathing (listening)
3. **Bot responds**: "Of course, I'll set up some warm ambient lighting for you"
4. **LED changes**: Slow gradient through warm colors (red → orange → yellow)
5. **Bot adds**: "Let me know if you want it brighter or dimmer, or if you'd like me to stay quiet"
6. **User**: "Perfect, and don't talk to me unless I ask"
7. **Bot**: "Got it, I'll stay quiet. I'm here if you need anything"
8. **State**: Silent (conversation) + Ambient (lighting) - pretty lights, bot won't initiate conversation

**Alternative**: If user just wants lighting but occasional check-ins are ok:
- **State**: Ready (conversation) + Ambient (lighting) - pretty lights, bot might still say "goodnight" or check in occasionally

---

### Scenario 3: Music Mode Detection

**Context**: Microphone detects rhythmic audio patterns (music playing)

1. **Bot detects**: Audio analysis shows music playing
2. **Bot observes**: Camera shows user at desk, presence confirmed
3. **Bot decides**: 30% chance to offer music lights
4. **LED changes**: Subtle pulse to indicate "I have an idea"
5. **Bot asks**: "I hear music, want me to put on a light show?"
6. **User**: "Yes!"
7. **LED changes**: Rainbow cycle or color-reactive pattern
8. **Mode**: Music mode - pattern stays until song ends or user requests change

---

### Scenario 4: Pre-Meeting Do Not Disturb

**Context**: User about to join work meeting

1. **User says**: "Hey Bot, shut up for a while, I'm about to be in a meeting"
2. **LED changes**: Orange breathing (listening)
3. **Bot responds**: "Got it! I'll sit quietly so I don't disturb you. Let me know when you're done"
4. **LED changes**: Dim white → Off
5. **State**: Silent (conversation) + Minimal (lighting) - bot won't talk, lights off
6. **User (later)**: "Hey Bot, meeting's over"
7. **Bot**: "Welcome back! How did it go?"
8. **State**: Returns to Ready + State-based (normal operation)

---

### Scenario 6: Ambient Lighting While Coding

**Context**: User wants nice lighting while coding, but bot can still check in

1. **User says**: "Hey Bot, give me some cool ambient lighting for focused work"
2. **LED changes**: Orange breathing (listening)
3. **Bot responds**: "Setting up cool ambient lighting. I'll still keep an eye on you"
4. **LED changes**: Slow gradient through cool colors (blue → cyan → purple)
5. **State**: Ready (conversation) + Ambient (lighting)
6. **Later, bot observes**: User has been coding for 2 hours
7. **Bot decides**: Initiate conversation (ambient pattern continues)
8. **Bot says**: "You've been coding for a while, want to take a quick break?"
9. **User**: "Good idea, thanks!"
10. **LED**: Stays in ambient mode (cool gradient continues during conversation)

---

### Scenario 5: Playful Personality

**Context**: Bot detects user hasn't moved in long time, slouching

1. **Bot observes**: Camera shows user stationary for 3+ hours
2. **Bot decides**: Use playful personality to encourage movement
3. **LED changes**: Orange pulse (getting attention)
4. **Bot says**: "Get up, lazy!" (playful tone)
5. **User laughs**: "Alright, alright"
6. **Bot**: "That's better! Your back will thank me later"
7. **LED changes**: Happy green pulse

---

## Persistent Memory System

### What Gets Remembered

- User preferences (favorite colors, lighting patterns, sleep schedule)
- Conversation topics (what user likes to talk about)
- Environmental patterns (usual desk hours, temperature preferences)
- Learned personality traits (how user responds to humor, formality level)
- Important facts user shares (e.g., "I have a cat named Luna")

### Memory Storage

- JSON file-based storage (simple, inspectable)
- Memory tiers:
  - **Short-term**: Last 10 conversation exchanges (RAM)
  - **Session**: Current day's interactions (disk)
  - **Long-term**: Persistent facts and patterns (disk)
- mem0 pattern implementation for retrieval

### Memory Usage in Conversation

- Relevant memories injected into LLM context
- Bot references past conversations naturally
- "Hey, how's Luna doing?" (remembers cat's name from weeks ago)
- "Want me to set up your usual warm lighting?"

---

## Feature Priority Matrix

### Phase 1 (MVP): Basic Interaction
**Goal**: Functional companion with conversation and lighting

✅ Required:
- Wake phrase detection (Vosk keyword spotting)
- Speech-to-text (Vosk recognition)
- Text-to-speech (Piper)
- LLM conversation (Llamafile with Qwen2.5 3B)
- RGB LED control with state-based patterns
- PIR motion detection for presence
- Basic persistent memory (conversation history)

❌ Optional:
- Camera vision (can defer to Phase 2)
- DHT11 environmental monitoring (can defer to Phase 2)
- LCD display (can defer to Phase 2)

---

### Phase 2: Environmental Awareness
**Goal**: Bot can observe and respond to environment

Required:
- Camera vision for human detection
- DHT11 temperature/humidity monitoring
- LCD display for status
- Enhanced memory system with fact extraction
- Passive observation mode with random conversation initiation

---

### Phase 3: Advanced Personality
**Goal**: Bot feels truly alive

Required:
- Emotion detection from speech tone
- Contextual humor and playfulness
- Music detection and reactive lighting
- Gesture recognition (via camera)
- Advanced memory with user modeling
- Personality adaptation over time

---

## Success Criteria

The bot is successful when:

1. **It feels alive**: Random observations make it feel like a roommate, not a tool
2. **Visual feedback is intuitive**: LED colors/patterns clearly communicate state without explanation
3. **Conversation is natural**: Not transactional, but genuinely engaging
4. **Memory works**: Bot remembers past conversations and builds on them
5. **Failure is graceful**: If a sensor fails, bot continues operating with degraded capabilities
6. **It's helpful**: Genuinely improves your daily desk experience

The ultimate test: Would you miss it if it was gone?
