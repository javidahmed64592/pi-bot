//! # Observation Mode
//!
//! Passive observation logic for bot-initiated conversations.
//!
//! ## Overview
//!
//! When Pi Bot is in the `Ready` state and the user is present, it waits for a
//! randomly-chosen interval (configured via `behavior.passive_observation_interval`)
//! before transitioning to the `Observing` state. It then collects context about
//! the current situation and decides — using a weighted random probability — whether
//! to initiate a proactive conversation.
//!
//! ## Decision Model
//!
//! The probability of initiating is weighted by:
//! - **Presence duration**: longer at desk → higher chance
//! - **Time since last interaction**: longer silence → higher chance
//!
//! This keeps the bot from being overly chatty early in a session while making it
//! more likely to check in after long periods of silence.
//!
//! ## Extensibility
//!
//! [`ObservationContext`] is designed to grow. Future observation sources
//! (environmental readings, camera context, calendar awareness, etc.) can be added
//! as fields. The [`ObservationContext::build_opener_prompt`] method assembles all
//! available context into the LLM prompt, so new fields are automatically included.

use chrono::Local;
use std::time::Duration;

// ============================================================================
// ObservationContext
// ============================================================================

/// Contextual snapshot gathered when the bot transitions into `Observing` state.
///
/// This struct is intentionally open for extension: as new sensor inputs become
/// available (temperature, camera, calendar, etc.), add them here and update
/// [`build_opener_prompt`](ObservationContext::build_opener_prompt) to include
/// them in the generated LLM context.
#[derive(Debug, Clone)]
pub struct ObservationContext {
    /// How long the user has been continuously present at their desk (minutes)
    pub presence_duration_minutes: u32,

    /// Current time of day as a human-readable string (e.g. "2:30 PM")
    pub time_of_day: String,

    /// How many minutes have elapsed since the bot last spoke to the user
    pub minutes_since_last_interaction: u32,

    /// Recent facts from long-term memory relevant to the current moment.
    ///
    /// These are short text strings (e.g. "User likes coffee", "User is learning Rust").
    /// Populated from semantic memory when available; empty otherwise.
    pub recent_facts: Vec<String>,
    // Future extensions:
    // pub temperature_celsius: Option<f32>,
    // pub humidity_percent: Option<f32>,
    // pub calendar_event: Option<String>,
    // pub desk_objects_changed: bool,
}

impl ObservationContext {
    /// Create an `ObservationContext` from raw timing data.
    ///
    /// # Arguments
    /// * `presence_duration_minutes` - Minutes user has been continuously present
    /// * `time_since_last_interaction` - Duration elapsed since last bot speech
    /// * `recent_facts` - Facts from long-term memory (can be empty)
    pub fn new(
        presence_duration_minutes: u32,
        time_since_last_interaction: Duration,
        recent_facts: Vec<String>,
    ) -> Self {
        let minutes_since_last_interaction = (time_since_last_interaction.as_secs() / 60) as u32;
        let time_of_day = Local::now().format("%-I:%M %p").to_string();

        Self {
            presence_duration_minutes,
            time_of_day,
            minutes_since_last_interaction,
            recent_facts,
        }
    }

    /// Decide probabilistically whether the bot should initiate a conversation.
    ///
    /// The probability is weighted by presence duration and time since last
    /// interaction, with diminishing returns to avoid the bot becoming annoying.
    ///
    /// The formula and weights are configured in `config.yaml` under
    /// `behavior.observation_probability`.
    pub fn should_initiate(&self, config: &bot_core::config::ObservationProbabilityConfig) -> bool {
        let base = config.base;

        // More likely the longer the user has been at their desk
        let presence_steps = (self.presence_duration_minutes as f32
            / config.presence.minutes_per_step)
            .min(config.presence.max_steps);
        let presence_bonus = presence_steps * config.presence.bonus_per_step;

        // More likely the longer since we last talked
        let interaction_steps = (self.minutes_since_last_interaction as f32
            / config.interaction.minutes_per_step)
            .min(config.interaction.max_steps);
        let interaction_bonus = interaction_steps * config.interaction.bonus_per_step;

        let probability = (base + presence_bonus + interaction_bonus).min(config.ceiling);

        log::debug!(
            "[Observation] Initiation probability: {:.0}% (presence={}min, since_interaction={}min)",
            probability * 100.0,
            self.presence_duration_minutes,
            self.minutes_since_last_interaction,
        );

        rand::random::<f32>() < probability
    }

    /// Build the internal prompt used to ask the LLM to generate a conversation opener.
    ///
    /// This prompt is submitted as though it were user input so that the standard
    /// `LlmService::generate` path (with memory history) is reused without modification.
    /// The LLM's system prompt instructs it to be Pi Bot, so framing this as an
    /// internal instruction naturally produces a bot-initiated opener.
    ///
    /// When new context fields are added to `ObservationContext`, extend this method
    /// to include them so the LLM can reference them in the opener.
    pub fn build_opener_prompt(&self) -> String {
        let mut prompt = format!(
            "[Internal context — do not repeat this back]\n\
             You have decided to start a conversation on your own initiative.\n\
             Current time: {time}\n\
             User has been at their desk for: {presence} minutes\n\
             Minutes since you last spoke: {since}\n",
            time = self.time_of_day,
            presence = self.presence_duration_minutes,
            since = self.minutes_since_last_interaction,
        );

        if !self.recent_facts.is_empty() {
            prompt.push_str("Things you know about the user:\n");
            for fact in &self.recent_facts {
                prompt.push_str(&format!("  - {fact}\n"));
            }
        }

        prompt.push_str(
            "\nGenerate a single short, natural conversation opener (1-2 sentences). \
             Be warm, curious, or playful — whatever fits the moment. \
             Do not mention that you decided to talk; just speak naturally as you would.",
        );

        prompt
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_initiate_returns_bool() {
        let ctx = ObservationContext::new(
            60,
            Duration::from_secs(20 * 60),
            vec!["User likes coffee".to_string()],
        );
        // Create a default config for testing
        let config = bot_core::config::ObservationProbabilityConfig {
            base: 0.20,
            presence: bot_core::config::ProbabilityStepConfig {
                minutes_per_step: 30.0,
                max_steps: 4.0,
                bonus_per_step: 0.10,
            },
            interaction: bot_core::config::ProbabilityStepConfig {
                minutes_per_step: 15.0,
                max_steps: 3.0,
                bonus_per_step: 0.10,
            },
            ceiling: 0.90,
        };
        // Just verify it returns without panic; the value is probabilistic
        let _ = ctx.should_initiate(&config);
    }

    #[test]
    fn test_opener_prompt_contains_context() {
        let ctx = ObservationContext::new(
            45,
            Duration::from_secs(25 * 60),
            vec!["User is learning Rust".to_string()],
        );
        let prompt = ctx.build_opener_prompt();
        assert!(prompt.contains("45 minutes"));
        assert!(prompt.contains("User is learning Rust"));
    }

    #[test]
    fn test_opener_prompt_no_facts() {
        let ctx = ObservationContext::new(10, Duration::from_secs(5 * 60), vec![]);
        let prompt = ctx.build_opener_prompt();
        assert!(!prompt.contains("Things you know"));
    }

    #[test]
    fn test_time_of_day_set() {
        let ctx = ObservationContext::new(0, Duration::ZERO, vec![]);
        assert!(!ctx.time_of_day.is_empty());
    }
}
