//! Speaker Actuator Task
//!
//! Handles text-to-speech playback using Piper TTS and speaker controller.
//!
//! # Architecture
//! - Consumes Command::Speak from controller
//! - Generates audio with Piper TTS
//! - Plays audio through SpeakerController
//! - Handles graceful shutdown

use actuators::SpeakerController;
use anyhow::Result;
use audio_pipeline::PiperTts;
use bot_core::{Command, Event, SystemConfig};
use tokio::sync::{broadcast, mpsc};

/// Run speaker actuator task
///
/// # Arguments
/// * `config` - System configuration with TTS settings
/// * `command_rx` - Channel to receive commands from controller
/// * `event_tx` - Channel to send events back to controller (e.g., SpeechComplete)
/// * `shutdown_rx` - Shutdown signal receiver
///
/// # Behavior
/// Listens for Speak commands, generates audio with Piper TTS, and plays
/// through the speaker. Sends SpeechComplete event when playback finishes.
/// Handles StopSpeaking command to interrupt playback.
/// Exits gracefully on shutdown signal.
pub async fn run_speaker_actuator(
    config: &SystemConfig,
    mut command_rx: mpsc::Receiver<Command>,
    event_tx: mpsc::Sender<Event>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    log::info!("[Speaker Actuator Task] Starting...");

    // Initialize Piper TTS
    let mut tts = PiperTts::new(config.audio.piper.clone()).await?;

    // Initialize speaker controller
    let mut speaker = SpeakerController::new(&config.audio.speaker_device)?;

    log::info!(
        "[Speaker Actuator Task] Initialized (voice: {})",
        config.audio.piper.voice
    );

    loop {
        tokio::select! {
            // Receive commands
            Some(command) = command_rx.recv() => {
                match command {
                    Command::Speak { text } => {
                        log::info!("[Speaker Actuator Task] Speaking: '{}'", text);

                        // Generate audio with Piper TTS
                        match tts.synthesize(&text).await {
                            Ok(audio_data) => {
                                // Notify controller that playback is starting
                                if let Err(e) = event_tx.send(Event::SpeechPlaybackStarted).await {
                                    log::error!("[Speaker Actuator Task] Failed to send SpeechPlaybackStarted event: {}", e);
                                }

                                // Play audio through speaker
                                if let Err(e) = speaker.play(&audio_data) {
                                    log::error!("[Speaker Actuator Task] Failed to play audio: {}", e);
                                    // Send completion event even on error
                                    if let Err(e) = event_tx.send(Event::SpeechComplete).await {
                                        log::error!("[Speaker Actuator Task] Failed to send SpeechComplete event: {}", e);
                                    }
                                } else {
                                    // Wait for audio playback to finish (blocking)
                                    speaker.wait_for_completion();
                                    log::debug!("[Speaker Actuator Task] Playback complete");

                                    // Notify controller that speech is complete
                                    if let Err(e) = event_tx.send(Event::SpeechComplete).await {
                                        log::error!("[Speaker Actuator Task] Failed to send SpeechComplete event: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("[Speaker Actuator Task] TTS failed: {}", e);
                                // Still send completion event even on error so controller doesn't hang
                                if let Err(e) = event_tx.send(Event::SpeechComplete).await {
                                    log::error!("[Speaker Actuator Task] Failed to send SpeechComplete event: {}", e);
                                }
                            }
                        }
                    }

                    Command::StopSpeaking => {
                        log::info!("[Speaker Actuator Task] Stopping playback");
                        speaker.stop();
                    }

                    _ => {
                        // Ignore commands not relevant to speaker
                        log::trace!("[Speaker Actuator Task] Ignoring command: {:?}", command);
                    }
                }
            }

            // Shutdown signal
            _ = shutdown_rx.recv() => {
                log::info!("[Speaker Actuator Task] Shutdown signal received");
                speaker.stop();
                break;
            }
        }
    }

    log::info!("[Speaker Actuator Task] Stopped");
    Ok(())
}
