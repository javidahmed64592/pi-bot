# Pi Bot - State Model Specification

## Overview

This document defines Pi Bot's state model that separates **conversation behavior** from **lighting presentation**.

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
- → **Active**: Wake phrase detected
- → **Silent**: User requests Do Not Disturb

**Default lighting**: StateBased (minimal/off)

---

### 2. Observing
**What it means**: Bot noticed something interesting and is deciding whether to speak

**Behavior**:
- Evaluates context (user busy? appropriate time? presence?)
- Rolls dice (20% chance to initiate conversation)
- If yes: generates observation-based opener → Active
- If no: returns to Ready

**Duration**: Brief (2-5 seconds max)

**Transitions**:
- → **Active**: Bot decides to speak
- → **Ready**: Bot decides not to speak

**LED hint**: Subtle dim pulse (if StateBased), otherwise no change

---

### 3. Active (In Conversation)
**What it means**: Bot is actively engaged in conversation

**Sub-states** (cycles through these):
- **Listening**: Capturing speech from microphone
- **Thinking**: Processing input through LLM
- **Learning**: Storing important memory (brief)
- **Speaking**: Playing TTS audio

**Flow**:
```
Listening → Thinking → [Learning] → Speaking → Listening → ...
```

**Exit condition**: 10 seconds of silence → Ready

**Transitions**:
- → **Listening**: User is speaking
- → **Ready**: 10s of silence OR conversation naturally ends
- → **Silent**: User requests Do Not Disturb mid-conversation

**LED behavior**:
- **StateBased mode**: Orange (listening) → Blue (thinking) → Green (speaking)
- **Ambient mode**: Pattern continues (optional: dim during conversation)

---

### 4. Silent (Do Not Disturb)
**What it means**: Bot won't initiate conversations, minimal responses only

**Behavior**:
- No conversation initiation (never enters Observing)
- Still monitors sensors passively
- Still responds to wake phrase, but concisely
- No follow-up questions or chatty behavior

**Duration**:
- User-specified (default: 1 hour)
- Or until user explicitly ends it

**Transitions**:
- → **Ready**: Timer expires OR user says "meeting's over" / similar
- → **Active**: User says wake phrase (for brief, essential responses only)

**Default lighting**: Minimal (dim or off)

---

## Lighting Modes

These determine **what you see** (LED color, pattern, brightness):

### 1. StateBased (Default)
**What it means**: LED color/pattern reflects conversation state

**Patterns**:
| Conversation State | LED Pattern |
|-------------------|-------------|
| Ready | Minimal/off (or very dim) |
| Observing | Subtle dim pulse |
| Active.Listening | Orange breathing animation |
| Active.Thinking | Blue pulsing |
| Active.Speaking | Solid green |
| Active.Learning | Brief purple flash |
| Silent | Dim white or off |

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

**Example 1: Bedtime**
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
