//! # Memory Service
//!
//! Manages conversation memory with short-term (RAM) and persistent (disk) storage.
//!
//! ## Architecture
//!
//! - **Short-term memory**: Last N exchanges kept in RAM for immediate context
//! - **Session storage**: Daily JSON files (YYYY-MM-DD.json) for conversation history
//! - **Long-term memory**: Extracted facts stored separately (Phase 2)
//!
//! ## Memory Layers
//!
//! 1. **Working Memory** (RAM): Current conversation (configurable size)
//! 2. **Session Memory** (Disk): All conversations for the current day
//! 3. **Long-term Memory** (Disk): Extracted facts with semantic search (Phase 2.5)
//!
//! ## Usage
//!
//! ```no_run
//! use ai_controller::{MemoryService, Message};
//! use bot_core::config::MemoryConfig;
//!
//! let config = MemoryConfig {
//!     session_storage: "data/sessions/".to_string(),
//!     long_term_storage: "data/memory/".to_string(),
//!     max_short_term: 10,
//!     fact_extraction_enabled: false,
//!     embeddings: Some(EmbeddingsConfig {
//!         model_path: "models/embeddings/all-MiniLM-L6-v2.onnx".to_string(),
//!         tokenizer_path: "models/embeddings/all-MiniLM-L6-v2_tokenizer.json".to_string(),
//!         dimensions: 384,
//!         enabled: true,
//!     }),
//!     search: SearchConfig {
//!         top_k: 5,
//!         min_similarity: 0.7,
//!         max_facts: 1000,
//!     },
//! };
//!
//! let mut memory = MemoryService::new(config).expect("Failed to init memory");
//!
//! // Add conversation exchanges
//! memory.add_exchange("Hello!", "Hi there! How can I help?");
//!
//! // Add facts manually
//! memory.add_fact("User likes coffee").await.unwrap();
//!
//! // Search for relevant facts
//! let facts = memory.search_facts("What drinks do I like?", 5, 0.7).await.unwrap();
//! ```

use anyhow::{Context, Result};
use bot_core::config::MemoryConfig;
use chrono::{Local, NaiveDate, Utc};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::embedding_service::{cosine_similarity, EmbeddingService};

// Re-export Message type for convenience
pub use crate::llm_service::Message;

// ============================================================================
// Session Storage Types
// ============================================================================

/// A single conversation exchange (user input + assistant response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    /// Unix timestamp of when the exchange occurred
    pub timestamp: i64,

    /// User's input message
    pub user_message: String,

    /// Assistant's response
    pub assistant_message: String,
}

/// A complete conversation session (typically one day)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Date of this session (YYYY-MM-DD)
    pub date: String,

    /// All exchanges in chronological order
    pub exchanges: Vec<Exchange>,
}

impl Session {
    /// Create a new empty session for a given date
    fn new(date: String) -> Self {
        Self {
            date,
            exchanges: Vec::new(),
        }
    }

    /// Add an exchange to the session
    fn add_exchange(&mut self, user_msg: String, assistant_msg: String) {
        self.exchanges.push(Exchange {
            timestamp: Local::now().timestamp(),
            user_message: user_msg,
            assistant_message: assistant_msg,
        });
    }
}

// ============================================================================
// Long-term Memory Types (Phase 2.5)
// ============================================================================

/// Source of a fact (how it was obtained)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FactSource {
    /// User explicitly told the bot
    UserTold,

    /// Extracted from conversation
    Conversation,

    /// Inferred from observations
    Observation,

    /// From environmental sensors
    Environmental,
}

/// A single fact stored in long-term memory with semantic embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    /// Unique identifier
    pub id: String,

    /// Raw fact text (e.g., "User likes coffee")
    pub text: String,

    /// 384-dimensional embedding vector for semantic search
    pub embedding: Vec<f32>,

    /// When the fact was created
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: chrono::DateTime<Utc>,

    /// How the fact was obtained
    pub source: FactSource,

    /// Optional category for organization
    pub category: Option<String>,

    /// How many times this fact has been retrieved
    pub relevance_count: u32,

    /// Confidence in fact accuracy (0.0-1.0)
    pub confidence: f32,
}

impl Fact {
    /// Create a new fact with embedding
    pub fn new(
        text: String,
        embedding: Vec<f32>,
        source: FactSource,
        category: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            text,
            embedding,
            timestamp: Utc::now(),
            source,
            category,
            relevance_count: 0,
            confidence: 1.0,
        }
    }
}

/// Long-term memory database containing all facts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactDatabase {
    /// All stored facts
    pub facts: Vec<Fact>,

    /// Metadata about the database
    pub metadata: FactDatabaseMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactDatabaseMetadata {
    /// Total number of facts
    pub total_facts: usize,

    /// Last updated timestamp
    pub last_updated: String,

    /// Embedding model used
    pub embedding_model: String,
}

impl FactDatabase {
    /// Create a new empty fact database
    fn new() -> Self {
        Self {
            facts: Vec::new(),
            metadata: FactDatabaseMetadata {
                total_facts: 0,
                last_updated: Utc::now().to_rfc3339(),
                embedding_model: "all-MiniLM-L6-v2".to_string(),
            },
        }
    }

    /// Update metadata
    fn update_metadata(&mut self) {
        self.metadata.total_facts = self.facts.len();
        self.metadata.last_updated = Utc::now().to_rfc3339();
    }
}

// ============================================================================
// MemoryService - Conversation Memory Management
// ============================================================================

/// Memory service for managing conversation history
///
/// Handles short-term (RAM), session (disk), and semantic long-term memory.
/// Automatically saves sessions to daily JSON files.
pub struct MemoryService {
    config: MemoryConfig,

    /// Short-term working memory (last N exchanges)
    short_term: Vec<Exchange>,

    /// Current session (today's full conversation history)
    current_session: Session,

    /// Path to session storage directory
    session_dir: PathBuf,

    /// Path to today's session file
    session_file_path: PathBuf,

    /// Long-term semantic memory (facts with embeddings)
    fact_database: FactDatabase,

    /// Path to fact database JSON file
    facts_file_path: PathBuf,

    /// Embedding service for semantic search (optional)
    embedder: Option<EmbeddingService>,
}

impl MemoryService {
    /// Create a new memory service and load today's session if it exists
    ///
    /// # Arguments
    ///
    /// * `config` - Memory configuration (storage paths, limits)
    ///
    /// # Returns
    ///
    /// Result containing MemoryService or error if initialization fails
    ///
    /// # Behavior
    ///
    /// - Creates storage directories if they don't exist
    /// - Loads today's session from disk if available
    /// - Loads fact database from disk if available
    /// - Initializes embedding service if configured
    /// - Initializes empty session/database if no existing data found
    pub fn new(config: MemoryConfig) -> Result<Self> {
        info!("Initializing memory service");

        // Create session storage directory if it doesn't exist
        let session_dir = PathBuf::from(&config.session_storage);
        fs::create_dir_all(&session_dir).context("Failed to create session storage directory")?;

        // Create long-term storage directory
        let long_term_dir = PathBuf::from(&config.long_term_storage);
        fs::create_dir_all(&long_term_dir)
            .context("Failed to create long-term storage directory")?;

        // Determine today's session file path
        let today = Local::now().format("%Y-%m-%d").to_string();
        let session_file_path = session_dir.join(format!("{}.json", today));

        // Determine facts file path
        let facts_file_path = long_term_dir.join("facts.json");

        // Load or create today's session
        let current_session = if session_file_path.exists() {
            info!("Loading existing session: {}", session_file_path.display());
            Self::load_session_from_file(&session_file_path).unwrap_or_else(|e| {
                warn!("Failed to load session, starting fresh: {}", e);
                Session::new(today.clone())
            })
        } else {
            info!("Creating new session for {}", today);
            Session::new(today)
        };

        // Initialize short-term memory with recent exchanges from session
        let short_term: Vec<Exchange> = current_session
            .exchanges
            .iter()
            .rev() // Most recent first
            .take(config.max_short_term)
            .rev() // Restore chronological order
            .cloned()
            .collect();

        // Load or create fact database
        let fact_database = if facts_file_path.exists() {
            info!("Loading fact database: {}", facts_file_path.display());
            Self::load_fact_database(&facts_file_path).unwrap_or_else(|e| {
                warn!("Failed to load fact database, starting fresh: {}", e);
                FactDatabase::new()
            })
        } else {
            info!("Creating new fact database");
            FactDatabase::new()
        };

        // Initialize embedding service if configured
        let embedder = if let Some(ref emb_config) = config.embeddings {
            if emb_config.enabled {
                info!("Initializing embedding service");
                match EmbeddingService::new(&emb_config.model_path, &emb_config.tokenizer_path) {
                    Ok(embedder) => {
                        info!("Embedding service initialized");
                        Some(embedder)
                    }
                    Err(e) => {
                        warn!("Failed to initialize embedding service: {}", e);
                        warn!("Semantic memory will be disabled");
                        None
                    }
                }
            } else {
                info!("Embedding service disabled in config");
                None
            }
        } else {
            info!("No embeddings config found, semantic memory disabled");
            None
        };

        info!(
            "Memory initialized: {} exchanges in session, {} in short-term, {} facts",
            current_session.exchanges.len(),
            short_term.len(),
            fact_database.facts.len()
        );

        Ok(Self {
            config,
            short_term,
            current_session,
            session_dir,
            session_file_path,
            fact_database,
            facts_file_path,
            embedder,
        })
    }

    /// Add a conversation exchange (user message + assistant response)
    ///
    /// # Arguments
    ///
    /// * `user_msg` - User's input message
    /// * `assistant_msg` - Assistant's response
    ///
    /// # Behavior
    ///
    /// - Adds exchange to short-term memory (trimming if needed)
    /// - Adds exchange to current session
    /// - Automatically saves session to disk
    pub fn add_exchange(&mut self, user_msg: impl Into<String>, assistant_msg: impl Into<String>) {
        let user_msg = user_msg.into();
        let assistant_msg = assistant_msg.into();

        debug!("Adding exchange to memory");

        // Create exchange
        let exchange = Exchange {
            timestamp: Local::now().timestamp(),
            user_message: user_msg.clone(),
            assistant_message: assistant_msg.clone(),
        };

        // Add to short-term memory
        self.short_term.push(exchange.clone());

        // Trim short-term memory if needed
        if self.short_term.len() > self.config.max_short_term {
            let trim_count = self.short_term.len() - self.config.max_short_term;
            self.short_term.drain(0..trim_count);
            debug!(
                "Trimmed {} old exchanges from short-term memory",
                trim_count
            );
        }

        // Add to current session
        self.current_session.add_exchange(user_msg, assistant_msg);

        // Auto-save session
        if let Err(e) = self.save_session() {
            warn!("Failed to save session: {}", e);
        }
    }

    /// Get conversation context for LLM (from short-term memory)
    ///
    /// # Returns
    ///
    /// Vector of Messages in chronological order (oldest to newest)
    ///
    /// # Behavior
    ///
    /// Returns messages from short-term memory, respecting max_short_term limit.
    /// Each exchange contributes 2 messages (user + assistant).
    pub fn get_context(&self) -> Vec<Message> {
        self.short_term
            .iter()
            .flat_map(|exchange| {
                vec![
                    Message::user(&exchange.user_message),
                    Message::assistant(&exchange.assistant_message),
                ]
            })
            .collect()
    }

    /// Get the number of exchanges in short-term memory
    pub fn short_term_size(&self) -> usize {
        self.short_term.len()
    }

    /// Get the total number of exchanges in the current session
    pub fn session_size(&self) -> usize {
        self.current_session.exchanges.len()
    }

    /// Clear short-term memory (keeps session intact)
    ///
    /// # Behavior
    ///
    /// Clears working memory but preserves the persistent session on disk.
    /// Useful for starting a fresh conversation context.
    pub fn clear_short_term(&mut self) {
        debug!("Clearing short-term memory");
        self.short_term.clear();
    }

    /// Get all exchanges from the current session
    ///
    /// # Returns
    ///
    /// Reference to all exchanges in chronological order
    pub fn get_session_history(&self) -> &[Exchange] {
        &self.current_session.exchanges
    }

    /// Save current session to disk
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    ///
    /// # Behavior
    ///
    /// Serializes the current session to JSON and writes to daily session file.
    /// Called automatically after each exchange.
    fn save_session(&self) -> Result<()> {
        debug!("Saving session to: {}", self.session_file_path.display());

        let file =
            File::create(&self.session_file_path).context("Failed to create session file")?;
        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, &self.current_session)
            .context("Failed to serialize session")?;

        debug!("Session saved successfully");
        Ok(())
    }

    /// Load a session from a JSON file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to session file
    ///
    /// # Returns
    ///
    /// Result containing Session or error if load fails
    fn load_session_from_file(path: &Path) -> Result<Session> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open session file: {}", path.display()))?;
        let reader = BufReader::new(file);

        let session: Session =
            serde_json::from_reader(reader).context("Failed to deserialize session")?;

        Ok(session)
    }

    /// Load a specific session by date (YYYY-MM-DD)
    ///
    /// # Arguments
    ///
    /// * `date` - Date string in YYYY-MM-DD format
    ///
    /// # Returns
    ///
    /// Result containing Session or error if not found
    ///
    /// # Usage
    ///
    /// Useful for reviewing past conversations or building context from history.
    pub fn load_session(&self, date: &str) -> Result<Session> {
        let session_file = self.session_dir.join(format!("{}.json", date));

        if !session_file.exists() {
            anyhow::bail!("No session found for date: {}", date);
        }

        Self::load_session_from_file(&session_file)
    }

    /// List all available session dates
    ///
    /// # Returns
    ///
    /// Vector of date strings (YYYY-MM-DD) sorted chronologically
    ///
    /// # Behavior
    ///
    /// Scans session storage directory and extracts dates from filenames.
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        let mut dates = Vec::new();

        for entry in fs::read_dir(&self.session_dir).context("Failed to read session directory")? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // Validate date format (YYYY-MM-DD)
                    if NaiveDate::parse_from_str(stem, "%Y-%m-%d").is_ok() {
                        dates.push(stem.to_string());
                    }
                }
            }
        }

        dates.sort();
        Ok(dates)
    }

    /// Get memory statistics
    ///
    /// # Returns
    ///
    /// Formatted string with memory usage stats
    pub fn stats(&self) -> String {
        format!(
            "Short-term: {}/{} exchanges | Session: {} exchanges | Facts: {} | Date: {}",
            self.short_term.len(),
            self.config.max_short_term,
            self.current_session.exchanges.len(),
            self.fact_database.facts.len(),
            self.current_session.date
        )
    }

    // ========================================================================
    // Long-term Semantic Memory (Phase 2.5)
    // ========================================================================

    /// Add a fact to long-term memory with embedding
    ///
    /// # Arguments
    ///
    /// * `text` - Fact text (e.g., "User likes coffee")
    /// * `source` - How the fact was obtained
    /// * `category` - Optional category for organization
    ///
    /// # Returns
    ///
    /// Result containing the created Fact or error
    ///
    /// # Behavior
    ///
    /// - Generates embedding for the fact text
    /// - Checks for duplicates (high similarity)
    /// - Stores fact in database
    /// - Automatically saves database to disk
    pub async fn add_fact(
        &mut self,
        text: impl Into<String>,
        source: FactSource,
        category: Option<String>,
    ) -> Result<Fact> {
        let text = text.into();

        debug!("Adding fact to long-term memory: {}", text);

        // Check if embedding service is available
        let embedder = self
            .embedder
            .as_mut()
            .context("Embedding service not available")?;

        // Generate embedding
        let embedding = embedder
            .embed(&text)
            .context("Failed to generate embedding")?;

        // Check for duplicate facts
        if let Some(similar) = self.find_similar_fact(&embedding, 0.9)? {
            info!(
                "Fact already exists (similarity={:.2}): {}",
                similar.1, similar.0.text
            );
            return Ok(similar.0);
        }

        // Create and store fact
        let fact = Fact::new(text, embedding, source, category);

        self.fact_database.facts.push(fact.clone());
        self.fact_database.update_metadata();

        // Save to disk
        self.save_fact_database()?;

        info!("Fact added: {}", fact.text);

        Ok(fact)
    }

    /// Search for facts using semantic similarity
    ///
    /// # Arguments
    ///
    /// * `query` - Search query text
    /// * `top_k` - Number of top results to return
    /// * `min_similarity` - Minimum cosine similarity threshold (0.0-1.0)
    ///
    /// # Returns
    ///
    /// Result containing vector of (Fact, similarity_score) tuples, sorted by relevance
    ///
    /// # Example
    ///
    /// ```no_run
    /// let results = memory.search_facts("What drinks do I like?", 5, 0.7).await?;
    /// for (fact, score) in results {
    ///     println!("{} (score: {:.2})", fact.text, score);
    /// }
    /// ```
    pub async fn search_facts(
        &mut self,
        query: &str,
        top_k: usize,
        min_similarity: f32,
    ) -> Result<Vec<(Fact, f32)>> {
        debug!("Searching facts for: {}", query);

        // Check if embedding service is available
        let embedder = self
            .embedder
            .as_mut()
            .context("Embedding service not available")?;

        // Generate query embedding
        let query_embedding = embedder
            .embed(query)
            .context("Failed to generate query embedding")?;

        // Compute similarities with all facts
        let mut scored_facts: Vec<(Fact, f32)> = self
            .fact_database
            .facts
            .iter()
            .map(|fact| {
                let similarity = cosine_similarity(&query_embedding, &fact.embedding);
                (fact.clone(), similarity)
            })
            .filter(|(_, score)| *score >= min_similarity)
            .collect();

        // Sort by similarity (descending)
        scored_facts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top K
        let results: Vec<(Fact, f32)> = scored_facts.into_iter().take(top_k).collect();

        // Increment relevance counters for retrieved facts
        for (fact, _) in &results {
            if let Some(stored_fact) = self
                .fact_database
                .facts
                .iter_mut()
                .find(|f| f.id == fact.id)
            {
                stored_fact.relevance_count += 1;
            }
        }

        // Save updated relevance counts
        if !results.is_empty() {
            self.save_fact_database()?;
        }

        debug!("Found {} relevant facts", results.len());

        Ok(results)
    }

    /// Find a similar fact in the database
    ///
    /// Used for deduplication when adding new facts.
    fn find_similar_fact(
        &self,
        embedding: &[f32],
        min_similarity: f32,
    ) -> Result<Option<(Fact, f32)>> {
        let mut best_match: Option<(Fact, f32)> = None;

        for fact in &self.fact_database.facts {
            let similarity = cosine_similarity(embedding, &fact.embedding);

            if similarity >= min_similarity {
                if let Some((_, best_score)) = &best_match {
                    if similarity > *best_score {
                        best_match = Some((fact.clone(), similarity));
                    }
                } else {
                    best_match = Some((fact.clone(), similarity));
                }
            }
        }

        Ok(best_match)
    }

    /// Get all facts in the database
    pub fn get_all_facts(&self) -> &[Fact] {
        &self.fact_database.facts
    }

    /// Get number of facts in long-term memory
    pub fn fact_count(&self) -> usize {
        self.fact_database.facts.len()
    }

    /// Remove a fact by ID
    pub fn remove_fact(&mut self, fact_id: &str) -> Result<()> {
        let initial_len = self.fact_database.facts.len();

        self.fact_database.facts.retain(|f| f.id != fact_id);

        if self.fact_database.facts.len() < initial_len {
            self.fact_database.update_metadata();
            self.save_fact_database()?;
            info!("Fact removed: {}", fact_id);
            Ok(())
        } else {
            anyhow::bail!("Fact not found: {}", fact_id)
        }
    }

    /// Save fact database to disk
    fn save_fact_database(&self) -> Result<()> {
        debug!(
            "Saving fact database to: {}",
            self.facts_file_path.display()
        );

        let file = File::create(&self.facts_file_path).context("Failed to create facts file")?;
        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, &self.fact_database)
            .context("Failed to serialize fact database")?;

        debug!("Fact database saved successfully");
        Ok(())
    }

    /// Load fact database from disk
    fn load_fact_database(path: &Path) -> Result<FactDatabase> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open fact database: {}", path.display()))?;
        let reader = BufReader::new(file);

        let database: FactDatabase =
            serde_json::from_reader(reader).context("Failed to deserialize fact database")?;

        Ok(database)
    }

    /// Check if semantic memory is enabled
    pub fn has_semantic_memory(&self) -> bool {
        self.embedder.is_some()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(temp_dir: &TempDir) -> MemoryConfig {
        MemoryConfig {
            session_storage: temp_dir.path().to_str().unwrap().to_string(),
            long_term_storage: temp_dir
                .path()
                .join("memories.json")
                .to_str()
                .unwrap()
                .to_string(),
            max_short_term: 3,
            fact_extraction_enabled: false,
            embeddings: None,
            search: bot_core::config::SearchConfig {
                top_k: 5,
                min_similarity: 0.7,
                max_facts: 1000,
            },
        }
    }

    #[test]
    fn test_memory_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(&temp_dir);

        let memory = MemoryService::new(config).expect("Failed to init memory");

        assert_eq!(memory.short_term_size(), 0);
        assert_eq!(memory.session_size(), 0);
    }

    #[test]
    fn test_add_exchange() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(&temp_dir);
        let mut memory = MemoryService::new(config).unwrap();

        memory.add_exchange("Hello", "Hi there!");

        assert_eq!(memory.short_term_size(), 1);
        assert_eq!(memory.session_size(), 1);

        let context = memory.get_context();
        assert_eq!(context.len(), 2); // user + assistant
    }

    #[test]
    fn test_short_term_trimming() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(&temp_dir); // max_short_term = 3
        let mut memory = MemoryService::new(config).unwrap();

        // Add 5 exchanges
        for i in 1..=5 {
            memory.add_exchange(format!("Message {}", i), format!("Response {}", i));
        }

        // Should only keep last 3 exchanges in short-term
        assert_eq!(memory.short_term_size(), 3);

        // But session should have all 5
        assert_eq!(memory.session_size(), 5);

        // Context should only include last 3 exchanges (6 messages)
        let context = memory.get_context();
        assert_eq!(context.len(), 6);
    }

    #[test]
    fn test_clear_short_term() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(&temp_dir);
        let mut memory = MemoryService::new(config).unwrap();

        memory.add_exchange("Hello", "Hi!");
        memory.clear_short_term();

        assert_eq!(memory.short_term_size(), 0);
        assert_eq!(memory.session_size(), 1); // Session preserved
    }

    #[test]
    fn test_session_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(&temp_dir);

        // Create memory and add exchanges
        {
            let mut memory = MemoryService::new(config.clone()).unwrap();
            memory.add_exchange("First", "Response 1");
            memory.add_exchange("Second", "Response 2");
        }

        // Load memory again (should restore session)
        {
            let memory = MemoryService::new(config).unwrap();
            assert_eq!(memory.session_size(), 2);
            assert_eq!(memory.short_term_size(), 2);
        }
    }

    #[test]
    fn test_list_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let config = test_config(&temp_dir);

        let mut memory = MemoryService::new(config).unwrap();

        // Add an exchange to trigger session file creation
        memory.add_exchange("Test", "Response");

        // Should have today's session
        let sessions = memory.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);

        let today = Local::now().format("%Y-%m-%d").to_string();
        assert_eq!(sessions[0], today);
    }
}
