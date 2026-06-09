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
    println!(
        "Commands: 'quit'/'exit'/'q' to exit, 'clear' to clear history, 'stats' for memory stats, 'facts' to list stored facts"
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

        if input.to_lowercase() == "facts" {
            let facts = memory.get_all_facts();
            if facts.is_empty() {
                println!("📭 No facts stored in long-term memory yet.");
            } else {
                println!("📚 Long-term memory ({} facts):", facts.len());
                for fact in facts {
                    println!(
                        "  [{}] {} (retrieved {} times)",
                        &fact.id[..8],
                        fact.text,
                        fact.relevance_count
                    );
                }
            }
            println!();
            continue;
        }

        // Generate response
        print!("Bot: [Thinking...");
        io::stdout().flush()?;

        let start = std::time::Instant::now();

        // Get conversation context augmented with relevant long-term facts
        let context = if memory.has_semantic_memory() {
            memory.get_context_with_facts(input).await
        } else {
            memory.get_context()
        };

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

                // Extract and store facts from the exchange
                if config.memory.fact_extraction_enabled && memory.has_semantic_memory() {
                    print!("  [Extracting facts...");
                    io::stdout().flush()?;

                    let facts = service.extract_facts(input, &response).await;

                    if facts.is_empty() {
                        println!(" none found]");
                    } else {
                        println!(" {} extracted]", facts.len());
                        for fact_text in &facts {
                            match memory
                                .add_fact(
                                    fact_text.clone(),
                                    ai_controller::FactSource::Conversation,
                                    None,
                                )
                                .await
                            {
                                Ok(_) => println!("  💾 Stored: {}", fact_text),
                                Err(_) => println!("  ℹ Duplicate: {}", fact_text),
                            }
                        }
                    }
                    println!();
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
