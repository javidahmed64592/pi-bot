# Pi Bot - State Model Specification

## Overview

This document defines Pi Bot's state model that separates **conversation behavior** from **lighting presentation**.

## Key Principles

1. **Presence-Aware Power Management**: Bot enters Silent mode when user leaves desk (PIR timeout), returns to Ready when user returns
2. **Manual vs Auto Silent**: Track whether Silent was user-requested or auto (PIR), behave differently on exit
3. **Bot-Initiated Speech**: Observing → Active(Speaking) directly (bot speaks first, doesn't wait for user)
4. **Always Responsive**: Bot responds to wake word even in Silent mode (just more concisely)
5. **Active Overrides Ambient**: Conversation states always show state-based colors, even if ambient lighting is configured

## Core Insight

The key design decision is to treat **conversation state** and **lighting mode** as **independent orthogonal dimensions**:

```
Bot State = Conversation State × Lighting Mode
```

This allows flexible combinations like:
- Ambient lighting while bot can still talk (coding scenario)
- Ambient lighting while bot stays silent (bedtime scenario)
- State-based lighting during conversation (default)
- Minimal lighting during meetings

---

## Conversation States

These determine **what the bot does** (when it talks, when it listens, when it's silent):

### 1. Ready (Default)
**What it means**: Bot is awake and monitoring, ready to respond

**Behavior**:
- Listens for wake phrase continuously
- Can randomly enter Observing state
- Responds immediately to wake phrase
- Low CPU usage

**Transitions**:
- → **Observing**: Random timer (5-15 min) OR interesting sensor event
- → **Active(Listening)**: Wake phrase detected
- → **Silent (auto)**: PIR timeout (no presence detected for idle_timeout)

**Default lighting**: StateBased (green breathing)

---

### 2. Observing
**What it means**: Bot noticed something interesting and is deciding whether to speak

**Behavior**:
- Evaluates context (user busy? appropriate time? presence?)
- Decides whether to initiate conversation
- If yes: generates observation-based opener → Active(Speaking)
- If no: returns to Ready

**Duration**: Brief (2-5 seconds max)

**Transitions**:
- → **Active(Speaking)**: Bot decides to speak (generates greeting/observation)
- → **Ready**: Bot decides not to speak

**LED hint**: Blue breathing (if StateBased), otherwise ambient pattern continues

---

### 3. Active (In Conversation)
**What it means**: Bot is actively engaged in conversation

**Sub-states** (flow depends on how conversation started):

**If user initiated (wake word)**:
```
Listening → Thinking → Learning → Speaking → Ready
```

**If bot initiated (Observing)**:
```
Speaking → Ready
(Bot speaks its observation, then waits for wake word to continue)
```

**Sub-state details**:
- **Listening**: Capturing speech from microphone (after wake word)
- **Thinking**: Processing input through LLM
- **Learning**: Storing important memory (brief)
- **Speaking**: Playing TTS audio

**Exit condition**: After Speaking completes → Ready (10s conversation timeout if in Listening)

**Transitions**:
- → **Ready**: After speaking OR 10s of silence in Listening
- → **Silent (manual)**: User requests Do Not Disturb mid-conversation

**LED behavior**:
- **StateBased mode**: Orange (listening) → Blue (thinking) → Purple (learning) → Green (speaking)
- **Ambient mode**: Active state OVERRIDES ambient (state-based colors during conversation)

---

### 4. Silent (Do Not Disturb)
**What it means**: Bot won't initiate conversations, minimal responses only

**Two modes**:
- **Auto Silent**: Entered via PIR timeout (no presence detected)
- **Manual Silent**: Entered via user DND request

**Behavior**:
- No conversation initiation (never enters Observing)
- Still monitors sensors passively (low power)
- Still responds to wake phrase, but with short, concise responses
- No follow-up questions or chatty behavior

**Transitions**:

**From Auto Silent (PIR triggered)**:
- → **Ready**: Presence detected (PIR senses user returned)
- → **Active(Listening)**: Wake phrase detected
- Optional: Bot greets user when transitioning to Ready

**From Manual Silent (user requested)**:
- → **Ready**: User explicitly requests wake up
- → **Active(Listening)**: Wake phrase detected
- Does NOT auto-exit on presence (user must explicitly wake bot)

**Default lighting**: StateBased (red breathing)

---

## Lighting Modes

These determine **what you see** (LED color, pattern, brightness):

### 1. StateBased (Default)
**What it means**: LED color/pattern reflects conversation state

**Patterns**:
| Conversation State | LED Pattern |
|-------------------|-------------|
| Ready | Green breathing |
| Observing | Blue breathing |
| Active.Listening | Orange breathing animation |
| Active.Thinking | Blue pulsing |
| Active.Speaking | Solid green |
| Active.Learning | Purple pulsing |
| Silent | Red breathing |

**Use case**: Normal operation, clear visual feedback

---

### 2. Ambient
**What it means**: Decorative pattern independent of conversation state

**Pattern types**:
- **Gradient**: Smooth transition through color palette
- **Rainbow**: Full spectrum cycle
- **Pulse**: Breathing animation with specified color
- **Static**: Solid color

**Examples**:
- Warm gradient (red → orange → yellow) for comfort/bedtime
- Cool gradient (blue → cyan → purple) for focus/coding
- Rainbow cycle for celebration/music mode

**Use case**: User wants aesthetic lighting

**Behavior during conversation**:
- Pattern continues by default
- Optional: slightly dim or pause pattern during Active state
- User-configurable preference

---

### 3. Minimal
**What it means**: LED is very dim or completely off

**Use case**:
- Silent mode (meetings, sleep)
- User prefers no visual distraction
- Power saving mode

---

## Status LEDs (Green & Red)

The bot has **4 additional status LEDs** independent of the main RGB LED: **2 green LEDs** and **2 red LEDs**. These provide quick at-a-glance status indication without interfering with RGB mood/ambient lighting.

### Design Philosophy

**Mutual Exclusivity**: Only one set (green OR red) is active at any time
- Green LEDs = Bot is active/available
- Red LEDs = Bot is idle/unavailable/error

This creates clear visual language: "green = go" and "red = stop"

### Green LEDs (Active State Indicators)

**Purpose**: Show bot is ready and available for interaction

| Conversation State | Green LED Pattern |
|-------------------|-------------------|
| **Ready** | Solid green (both LEDs) |
| **Observing** | Solid green (both LEDs) |
| **Active.Listening** | Breathing green |
| **Active.Thinking** | Breathing green |
| **Active.Speaking** | Breathing green |
| **Active.Learning** | Breathing green |
| **Silent** | Off (red LEDs active instead) |

**Key Points**:
- Solid = Ready to interact (waiting for wake phrase)
- Breathing = Processing something (listening, thinking, speaking)
- Green LEDs provide feedback even when RGB is in Ambient mode
- User can quickly glance to see "green = I can talk to it"

### Red LEDs (Idle/Error State Indicators)

**Purpose**: Show bot is unavailable or has an error

| State | Red LED Pattern |
|-------|-----------------|
| **Silent (DND)** | Breathing red |
| **System Error** | Flashing red (fast) |
| **All other states** | Off (green LEDs active instead) |

**Key Points**:
- Breathing = Intentional silence (DND mode)
- Flashing = System error (component failure, critical issue)
- Red LEDs only active when green LEDs are off

### Status LED + RGB LED Combinations

The status LEDs work **in parallel** with RGB LED states:

| Use Case | Conversation | RGB LED Mode | Status LEDs |
|----------|-------------|--------------|-------------|
| **Normal ready** | Ready | StateBased (minimal) | Green solid |
| **Coding with ambiance** | Ready | Ambient (cool gradient) | Green solid |
| **Bot listening** | Active.Listening | StateBased (orange breathing) | Green breathing |
| **Ambient + talking** | Active.Speaking | Ambient (continues) | Green breathing |
| **DND/Silent mode** | Silent | Ambient (warm) | Red breathing |
| **Meeting mode** | Silent | Minimal | Red breathing |
| **System error** | Any | StateBased (red) | Red flashing |

**Benefits**:
- Status LEDs provide consistent feedback regardless of RGB mode
- RGB can show mood/ambiance while status LEDs show availability
- No confusion: green = available, red = unavailable

### Implementation Patterns

**Pattern: Breathing**
```
- Smooth fade in/out cycle
- Period: ~2-3 seconds per cycle
- Used for: Active processing (green) or intentional DND (red)
```

**Pattern: Flashing**
```
- Quick on/off toggle
- Period: ~500ms on, 500ms off
- Used for: System errors (red only)
```

**Pattern: Solid**
```
- Constant brightness
- Used for: Ready state (green only)
```

---

## Complete State Transition Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Conversation States                       │
│                                                             │
│  ┌─────────┐                                               │
│  │ Ready   │◄──────────────────┐                          │
│  │ (green  │                    │                          │
│  │breathing)│                    │                          │
│  └────┬────┘                    │                          │
│       │                         │                          │
│       │ PIR timeout             │ Presence detected        │
│       │ (no motion)             │ (PIR)                    │
│       ▼                         │                          │
│  ┌──────────┐                   │                          │
│  │ Silent   │                   │                          │
│  │ (auto)   │───────────────────┘                          │
│  │ (red     │                                               │
│  │breathing)│                                               │
│  └────┬────┘                                               │
│       │                                                     │
│       │ User DND                                           │
│       │ request                                            │
│       ▼                                                     │
│  ┌──────────┐      User wake up request                   │
│  │ Silent   │──────────────────────────┐                  │
│  │ (manual) │                           │                  │
│  │ (red     │                           ▼                  │
│  │breathing)│                      ┌─────────┐            │
│  └──────────┘                      │ Ready   │            │
│                                     └─────────┘            │
│                                                             │
│  ┌─────────┐      Random/Interesting                      │
│  │ Ready   │──────────────────────┐                        │
│  └─────────┘                      │                        │
│       │                            ▼                        │
│       │ Wake word            ┌──────────┐                 │
│       │                      │Observing │                 │
│       ▼                      │ (blue    │                 │
│  ┌──────────────┐            │breathing)│                 │
│  │ Active       │            └─────┬────┘                 │
│  │ (Listening)  │                  │                        │
│  │ (orange      │◄─────────────────┘                        │
│  │ breathing)   │  Decides to speak                        │
│  └──────┬───────┘                  │                        │
│         │                          │ Decides not           │
│         │                          ▼                        │
│         │                     ┌─────────┐                 │
│         │                     │ Ready   │                 │
│         │                     └─────────┘                 │
│         │                                                   │
│         │ Speech                                           │
│         │ captured                                         │
│         ▼                                                   │
│  ┌──────────────┐                                          │
│  │ Active       │                                          │
│  │ (Thinking)   │                                          │
│  │ (blue pulse) │                                          │
│  └──────┬───────┘                                          │
│         │                                                   │
│         │ LLM response                                     │
│         ▼                                                   │
│  ┌──────────────┐                                          │
│  │ Active       │                                          │
│  │ (Learning)   │                                          │
│  │(purple pulse)│                                          │
│  └──────┬───────┘                                          │
│         │                                                   │
│         │ Memory saved                                     │
│         ▼                                                   │
│  ┌──────────────┐                                          │
│  │ Active       │                                          │
│  │ (Speaking)   │◄─────────────────────────────────────┐  │
│  │ (green solid)│  Bot decides to speak (Observing)    │  │
│  └──────┬───────┘                                       │  │
│         │                                                │  │
│         │ TTS complete                                   │  │
│         ▼                                                │  │
│    ┌─────────┐                                          │  │
│    │ Ready   │──────────────────────────────────────────┘  │
│    └─────────┘                                             │
│                                                             │
│  Note: Wake word works in ANY state (including Silent)     │
│        Active state ALWAYS overrides ambient lighting      │
└─────────────────────────────────────────────────────────────┘
```

---

## State Combinations

### Common Scenarios

| Scenario | Conversation | Lighting | Behavior |
|----------|-------------|----------|----------|
| **Just turned on** | Ready | StateBased | Default state, waiting for input |
| **Normal usage** | Ready | StateBased | Can talk, LED shows state |
| **Coding with ambiance** | Ready | Ambient (cool) | Pretty lights, bot can check in |
| **Bedtime lighting** | Silent | Ambient (warm) | Pretty lights, bot stays quiet |
| **Meeting mode** | Silent | Minimal | Completely unobtrusive |
| **Active chat** | Active | StateBased | Conversation with visual feedback |
| **Music mode** | Ready | Ambient (rainbow) | Pretty lights, bot available |

### State Transition Examples

**Example 1: User Leaves Desk**
```
1. Initial state: Ready (green breathing, presence detected)

2. User walks away for coffee (PIR detects no motion)
   After idle_timeout (default 30 min): Ready → Silent (auto)
   State: Silent (auto) + Red breathing

3. User returns to desk (PIR detects motion)
   Immediately: Silent (auto) → Ready
   State: Ready + Green breathing
   Optional: Bot says "Welcome back!"
```

**Example 2: Bot Initiates Conversation**
```
1. User is working (Ready state, presence detected)
   State: Ready + Green breathing

2. Bot's camera notices interesting activity
   State: Ready → Observing
   LED: Green → Blue breathing (brief)

3. Bot decides to speak (generates observation)
   State: Observing → Active(Speaking)
   LED: Blue → Green solid
   Bot: "I noticed you're looking at travel photos. Planning a trip?"

4. Speaking completes
   State: Active(Speaking) → Ready
   LED: Green solid → Green breathing
   (Bot waits for wake word if user wants to respond)
```

**Example 3: User Conversation**
```
1. User says "Hey Bot"
   State: Ready → Active(Listening)
   LED: Green breathing → Orange breathing

2. User: "What's the weather tomorrow?"
   State: Active(Listening) → Active(Thinking)
   LED: Orange breathing → Blue pulse
   (Bot queries LLM)

3. LLM responds, bot stores exchange
   State: Active(Thinking) → Active(Learning)
   LED: Blue pulse → Purple pulse (brief)

4. Bot starts speaking
   State: Active(Learning) → Active(Speaking)
   LED: Purple pulse → Green solid
   Bot: "Tomorrow will be sunny, high of 22°C"

5. Speaking completes
   State: Active(Speaking) → Ready
   LED: Green solid → Green breathing
```

**Example 4: Bedtime (Manual DND)**
```
1. User: "Hey Bot, I'm going to bed, don't disturb me"
   State: Ready → Silent (manual)
   LED: Green breathing → Red breathing

2. User sleeps (8 hours pass, no presence)
   State: Remains Silent (manual) - does NOT auto-exit on PIR

3. User wakes up and says "Hey Bot, good morning"
   Bot responds but stays in Silent mode (concise response)
   State: Remains Silent (manual) after response

4. User: "Hey Bot, you can talk normally now"
   State: Silent (manual) → Ready
   LED: Red breathing → Green breathing
```

**Example 5: Coding with Ambient Lighting**
```
1. User: "Hey Bot, light up the room but don't talk to me"
   State: Ready + StateBased

2. Bot: "Setting up ambient lighting. I'll stay quiet unless you need me."
   State: Silent + Ambient (warm gradient)

3. Bot continues showing warm lights, won't initiate conversation
   Responds only if user says "Hey Bot"
```

**Example 2: Coding**
```
1. User: "Hey Bot, give me cool ambient lighting for focused work"
   State: Ready + StateBased

2. Bot: "Setting up cool lighting. I'll still keep an eye on you."
   State: Ready + Ambient (cool gradient)

3. [2 hours later] Bot: "You've been coding for a while, take a break?"
   State: Active + Ambient (lights continue during conversation)

4. User: "Good idea!" [conversation ends]
   State: Ready + Ambient (lights persist after conversation)
```

**Example 3: Meeting**
```
1. User: "Hey Bot, I'm in a meeting"
   State: Ready + StateBased

2. Bot: "Got it, I'll be quiet."
   State: Silent + Minimal

3. [1 hour later, auto-timeout]
   State: Ready + StateBased (returns to default)
```

---

## Implementation Notes

### BotState Structure

```rust
pub struct BotState {
    // Primary state dimensions
    pub conversation_state: ConversationState,
    pub lighting_mode: LightingMode,

    // Context tracking
    pub presence_detected: bool,
    pub last_interaction: Instant,
    pub observing_since: Option<Instant>,
    pub current_emotion: Emotion,

    // Configuration
    pub ambient_persists_during_conversation: bool,
}
```

### State Machine Transitions

```rust
impl BotState {
    pub fn can_initiate_conversation(&self) -> bool {
        matches!(self.conversation_state, ConversationState::Ready)
    }

    pub fn should_show_state_in_led(&self) -> bool {
        matches!(self.lighting_mode, LightingMode::StateBased)
    }

    pub fn get_led_command_for_state(&self) -> Command {
        match (&self.conversation_state, &self.lighting_mode) {
            (ConversationState::Active(ActiveSubState::Listening), LightingMode::StateBased) => {
                Command::SetPattern {
                    pattern: LedPattern::Breathing,
                    colors: vec![RgbColor::orange()],
                }
            }
            (_, LightingMode::Ambient(pattern)) => {
                Command::SetAmbientPattern(pattern.clone())
            }
            // ... other combinations
        }
    }
}
```

### Configuration Options

Users can configure:
1. **Observation frequency**: How often bot enters Observing state (5-15 min default)
2. **Observation probability**: Chance to speak when observing (20% default)
3. **Silent duration**: Default Do Not Disturb timeout (1 hour default)
4. **Ambient persistence**: Whether ambient lighting continues during Active state
5. **Default lighting**: StateBased or Ambient on startup

### Customizing RGB LED Colors

All state colors and patterns are fully configurable in `config/config.yaml`. Each conversation state can have a custom color and pattern:

```yaml
rgb_led:
  ready:
    pattern: "breathing"
    color: [0, 255, 0]  # Green - change to your preference!

  observing:
    pattern: "breathing"
    color: [0, 0, 255]  # Blue

  silent:
    pattern: "breathing"
    color: [255, 0, 0]  # Red

  active:
    listening:
      pattern: "breathing"
      color: [255, 165, 0]  # Orange

    thinking:
      pattern: "pulse"
      color: [0, 0, 255]  # Blue

    speaking:
      pattern: "solid"
      color: [0, 255, 0]  # Green

    learning:
      pattern: "pulse"
      color: [128, 0, 128]  # Purple
```

**Available patterns**: `breathing`, `pulse`, `solid`, `gradient`, `rainbow`

**Color format**: `[R, G, B]` where each value is 0-255

---

## Testing Strategy

### Unit Tests

Test state transitions:
```rust
#[test]
fn test_wake_word_transitions_to_active() {
    let mut state = BotState::default();
    assert!(matches!(state.conversation_state, ConversationState::Ready));

    state.handle_wake_word();
    assert!(matches!(state.conversation_state,
        ConversationState::Active(ActiveSubState::Listening)));
}

#[test]
fn test_ambient_lighting_persists_across_conversation() {
    let mut state = BotState::default();
    state.lighting_mode = LightingMode::Ambient(AmbientPattern::Gradient { .. });
    state.ambient_persists_during_conversation = true;

    state.handle_wake_word();
    assert!(matches!(state.lighting_mode, LightingMode::Ambient(_)));
}
```

### Integration Tests

Test state combinations:
```rust
#[tokio::test]
async fn test_bedtime_scenario() {
    // User: "light up the room but don't talk to me"
    let state = handle_bedtime_request().await;

    assert!(matches!(state.conversation_state, ConversationState::Silent));
    assert!(matches!(state.lighting_mode, LightingMode::Ambient(_)));

    // Bot should not initiate conversation
    assert!(!state.can_initiate_conversation());
}
```

---

## Future Enhancements

### Potential Additions

1. **Music-reactive lighting**: Ambient pattern synced to audio
2. **Notification mode**: Brief visual alerts during Silent state
3. **Scheduled states**: Automatic Silent mode during calendar events
4. **Learning preferences**: Bot learns when user prefers Silent vs Ready

### Staying Simple

**Resist temptation** to add:
- More conversation states (4 is enough)
- More lighting modes (3 covers all use cases)
- Complex state-dependent behaviors (keep orthogonal)

---

## Summary

**State model in one sentence**: Bot has a **conversation state** (Ready/Observing/Active/Silent) that determines when it talks, and a **lighting mode** (StateBased/Ambient/Minimal) that determines what you see, and these are **independent**.

**Default**: Ready + StateBased (bot can talk, LED shows state)

**User can request**: Any valid combination of conversation state and lighting mode

**Implementation**: Enums + state machine + command routing based on both dimensions
