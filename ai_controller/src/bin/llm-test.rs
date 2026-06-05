//! # LLM Test Binary
//!
//! Interactive CLI for testing the LLM service with llamafile.
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
//! 1. LLM API connectivity
//! 2. LLM initialization with configuration
//! 3. Single-turn and multi-turn conversations
//! 4. Response quality and latency
//! 5. Context management across exchanges
//!
//! ## Requirements
//!
//! - llamafile running locally (http://localhost:8080)
//! - Model embedded in llamafile or loaded via --model flag

use ai_controller::{LlmService, MemoryService};
use anyhow::{Context, Result};
use bot_core::config::load_config;
use log::{error, info};
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

    // Check API server health
    print!("Checking API server...");
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
            println!("Make sure llamafile is running:");
            println!("  1. Download llamafile for your model");
            println!("  2. Run: ./llamafile --server --port 8080");
            println!(
                "  3. Or run with specific model: ./llamafile --server --port 8080 -m model.gguf"
            );
            return Ok(());
        }
    }

    println!();
    println!("Ready for conversation! Type your messages below.");
    println!(
        "Commands: 'quit'/'exit'/'q' to exit, 'clear' to clear history, 'stats' for memory stats"
    );
    println!("─────────────────────────────────────────────────────");
    println!();

    // Initialize memory service
    let mut memory =
        MemoryService::new(config.memory.clone()).context("Failed to initialize memory service")?;

    info!("Memory initialized: {}", memory.stats());
    if memory.session_size() > 0 {
        println!(
            "📝 Resuming session with {} previous exchanges",
            memory.session_size()
        );
        println!();
    }

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
            memory.clear_short_term();
            println!("✓ Short-term memory cleared");
            println!();
            continue;
        }

        if input.to_lowercase() == "stats" {
            println!("📊 {}", memory.stats());
            println!();
            continue;
        }

        // Generate response
        print!("Bot: [Thinking...");
        io::stdout().flush()?;

        let start = std::time::Instant::now();

        // Get conversation context from memory
        let context = memory.get_context();

        match service.generate(input, &context).await {
            Ok(response) => {
                let duration = start.elapsed();

                print!("\r"); // Clear "Thinking..." message
                print!("Bot: {}", response);
                println!();
                println!(
                    "  [Generated in {:.1}s, {} exchanges in context]",
                    duration.as_secs_f32(),
                    memory.short_term_size()
                );
                println!();

                // Add exchange to memory (auto-saves to disk)
                memory.add_exchange(input, &response);
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
