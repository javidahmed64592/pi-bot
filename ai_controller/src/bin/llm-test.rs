//! # LLM Test Binary
//!
//! Interactive CLI for testing the LLM service with Ollama.
//!
//! ## Usage
//!
//! ```bash
//! # Start interactive conversation
//! cargo run --bin llm-test
//!
//! # Type messages and press Enter
//! # Type 'quit', 'exit', or 'q' to exit
//! # Type 'clear' to clear conversation history
//! ```
//!
//! ## What it tests
//!
//! 1. Ollama API connectivity
//! 2. LLM initialization with configuration
//! 3. Single-turn and multi-turn conversations
//! 4. Response quality and latency
//! 5. Context management across exchanges
//!
//! ## Requirements
//!
//! - Ollama running locally (http://localhost:11434)
//! - Qwen2.5 7B model downloaded: `ollama pull qwen2.5:7b-instruct`

use ai_controller::{LlmService, Message};
use anyhow::{Context, Result};
use bot_core::config::load_config;
use log::error;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== Pi Bot LLM Test ===");
    println!();

    // Load configuration
    let config = load_config("config/config.yaml").context("Failed to load configuration")?;

    // Initialize LLM service
    println!("Initializing LLM service...");
    let service = LlmService::new(config.llm.clone())
        .await
        .context("Failed to initialize LLM service")?;

    println!("✓ LLM initialized: {}", service.model_name());
    println!("  Temperature: {}", service.temperature());
    println!();

    // Check Ollama health
    print!("Checking Ollama server...");
    io::stdout().flush()?;
    match service.check_health().await {
        Ok(true) => println!(" ✓ Connected"),
        Ok(false) => {
            println!(" ✗ Server returned error");
            return Ok(());
        }
        Err(e) => {
            println!(" ✗ Failed: {}", e);
            println!();
            println!("Make sure Ollama is running:");
            println!("  1. Install: https://ollama.com");
            println!("  2. Pull model: ollama pull qwen2.5:7b-instruct");
            println!("  3. Verify: ollama list");
            return Ok(());
        }
    }

    println!();
    println!("Ready for conversation! Type your messages below.");
    println!("Commands: 'quit'/'exit'/'q' to exit, 'clear' to clear history");
    println!("─────────────────────────────────────────────────────");
    println!();

    // Conversation loop
    let mut history: Vec<Message> = Vec::new();
    let mut exchange_count = 0;

    loop {
        // Get user input
        print!("You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // Handle commands
        if input.is_empty() {
            continue;
        }

        if matches!(input.to_lowercase().as_str(), "quit" | "exit" | "q") {
            println!();
            println!("Goodbye! 👋");
            break;
        }

        if input.to_lowercase() == "clear" {
            history.clear();
            exchange_count = 0;
            println!("✓ Conversation history cleared");
            println!();
            continue;
        }

        // Generate response
        print!("Bot: [Thinking...");
        io::stdout().flush()?;

        let start = std::time::Instant::now();

        match service.generate(input, &history).await {
            Ok(response) => {
                let duration = start.elapsed();

                print!("\r"); // Clear "Thinking..." message
                print!("Bot: {}", response);
                println!();
                println!(
                    "  [Generated in {:.1}s, {} exchanges in context]",
                    duration.as_secs_f32(),
                    exchange_count
                );
                println!();

                // Update history (keep last 10 exchanges)
                history.push(Message::user(input));
                history.push(Message::assistant(response));

                exchange_count += 1;

                // Trim history to prevent context overflow
                const MAX_HISTORY: usize = 10 * 2; // 10 exchanges = 20 messages
                if history.len() > MAX_HISTORY {
                    let trim_count = history.len() - MAX_HISTORY;
                    history.drain(0..trim_count);
                    exchange_count = 10;
                }
            }
            Err(e) => {
                error!("Failed to generate response: {}", e);
                println!("✗ Error: {}", e);
                println!();
            }
        }
    }

    Ok(())
}
