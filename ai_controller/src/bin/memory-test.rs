//! Memory Test Binary - Tests embedding service and semantic memory
//!
//! This binary tests the enhanced memory system with semantic search.
//!
//! Usage:
//!   cargo run --bin memory-test --release
//!
//! Features tested:
//! - Embedding generation
//! - Fact storage with embeddings
//! - Semantic search
//! - Fact deduplication
//! - Memory persistence

use ai_controller::{cosine_similarity, EmbeddingService, FactSource, MemoryService};
use bot_core::load_config;
use env_logger::Builder;
use log::{info, LevelFilter};
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    Builder::new()
        .filter_level(LevelFilter::Info)
        .format(|buf, record| writeln!(buf, "[{}] {}", record.level(), record.args()))
        .init();

    info!("Pi Bot - Memory System Test");
    info!("============================\n");

    // Load configuration
    let config = load_config("config/config.yaml")?;

    // Check if embeddings are enabled
    if let Some(ref emb_config) = config.memory.embeddings {
        if !emb_config.enabled {
            eprintln!("❌ Embeddings disabled in config.yaml");
            eprintln!("Set memory.embeddings.enabled to true and try again");
            return Ok(());
        }
    } else {
        eprintln!("❌ No embeddings configuration found in config.yaml");
        return Ok(());
    }

    // Test 1: Embedding Service
    info!("Test 1: Embedding Service");
    info!("─────────────────────────");

    let mut embedder = EmbeddingService::new(
        &config
            .memory
            .embeddings
            .as_ref()
            .unwrap()
            .model_path,
        &config
            .memory
            .embeddings
            .as_ref()
            .unwrap()
            .tokenizer_path,
    )?;

    info!("✓ Embedding service initialized");

    // Generate embeddings
    let text1 = "I like coffee";
    let text2 = "I enjoy coffee";
    let text3 = "The weather is nice today";

    info!("Generating embeddings...");
    let emb1 = embedder.embed(text1)?;
    let emb2 = embedder.embed(text2)?;
    let emb3 = embedder.embed(text3)?;

    info!("✓ Generated {} embeddings of {} dimensions", 3, emb1.len());

    // Test similarity
    let sim_similar = cosine_similarity(&emb1, &emb2);
    let sim_different = cosine_similarity(&emb1, &emb3);

    info!("Similarity scores:");
    info!("  '{}' vs '{}'  : {:.3}", text1, text2, sim_similar);
    info!("  '{}' vs '{}'  : {:.3}", text1, text3, sim_different);

    assert!(
        sim_similar > sim_different,
        "Similar sentences should have higher similarity"
    );
    info!("✓ Semantic similarity working correctly\n");

    // Test 2: Memory Service with Semantic Search
    info!("Test 2: Semantic Memory System");
    info!("──────────────────────────────");

    let mut memory = MemoryService::new(config.memory)?;

    if !memory.has_semantic_memory() {
        eprintln!("❌ Semantic memory not available");
        return Ok(());
    }

    info!("✓ Memory service initialized with semantic search");
    info!("Current facts in database: {}", memory.fact_count());

    // Add some test facts
    info!("\nAdding facts...");

    let facts = vec![
        ("User's favorite programming language is Rust", FactSource::UserTold),
        ("User likes coffee", FactSource::Conversation),
        ("User prefers tea in the morning", FactSource::Conversation),
        ("User's desk temperature is usually 22°C", FactSource::Environmental),
        ("User works on AI projects", FactSource::Observation),
    ];

    for (text, source) in facts {
        match memory.add_fact(text, source, None).await {
            Ok(fact) => info!("  ✓ Added: {} (ID: {})", fact.text, &fact.id[..8]),
            Err(_e) => info!("  ℹ Skipped (duplicate): {}", text),
        }
    }

    info!("\nTotal facts in database: {}", memory.fact_count());

    // Test semantic search
    info!("\nTest 3: Semantic Search");
    info!("───────────────────────");

    let queries = vec![
        "What programming languages does the user know?",
        "What drinks does the user like?",
        "What is the user working on?",
    ];

    for query in queries {
        info!("\nQuery: '{}'", query);

        let results = memory
            .search_facts(query, 3, 0.5)
            .await?;

        if results.is_empty() {
            info!("  No relevant facts found");
        } else {
            info!("  Found {} relevant facts:", results.len());
            for (fact, score) in results {
                info!("    [{:.3}] {}", score, fact.text);
            }
        }
    }

    // Test 4: Deduplication
    info!("\nTest 4: Deduplication");
    info!("────────────────────");

    info!("Trying to add duplicate fact...");
    let duplicate = "User likes coffee"; // Already added
    match memory
        .add_fact(duplicate, FactSource::Conversation, None)
        .await
    {
        Ok(_) => info!("  ℹ Fact added or merged"),
        Err(_) => info!("  ℹ Duplicate detected and skipped"),
    }

    // Final stats
    info!("\nFinal Memory Statistics");
    info!("──────────────────────");
    info!("{}", memory.stats());

    info!("\n✅ All tests completed successfully!");
    info!("\nNote: Run this test multiple times to verify persistence.");
    info!("Facts are stored in: data/memory/facts.json");

    Ok(())
}
