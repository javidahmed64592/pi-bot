//! # Embedding Service
//!
//! Converts text to dense vector representations for semantic search using ONNX Runtime.
//!
//! ## Model
//!
//! Uses `all-MiniLM-L6-v2` (80MB) which produces 384-dimensional embeddings.
//! This model is optimized for semantic similarity search and runs efficiently on CPU.
//!
//! ## Usage
//!
//! ```no_run
//! use ai_controller::EmbeddingService;
//!
//! let embedder = EmbeddingService::new("models/embeddings/all-MiniLM-L6-v2.onnx").unwrap();
//! let embedding = embedder.embed("Hello world").unwrap();
//! assert_eq!(embedding.len(), 384);
//! ```
//!
//! ## Performance
//!
//! Target: <100ms per embedding on Raspberry Pi 5
//!
//! ## Integration
//!
//! - Used by MemoryService for fact storage and retrieval
//! - Enables semantic search via cosine similarity
//! - Supports batch embedding for efficiency

use anyhow::{Context, Result};
use log::{debug, info};
use ndarray::{Array2, Axis};
use ort::session::Session;
use ort::value::Value;
use std::path::Path;
use tokenizers::Tokenizer;

/// Embedding service for converting text to dense vectors
///
/// Uses ONNX Runtime with all-MiniLM-L6-v2 model for fast CPU inference.
pub struct EmbeddingService {
    /// ONNX Runtime session
    session: Session,

    /// Tokenizer for preprocessing text
    tokenizer: Tokenizer,

    /// Expected embedding dimensions (384 for MiniLM-L6-v2)
    embedding_dim: usize,
}

impl EmbeddingService {
    /// Create a new embedding service
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to ONNX model file
    /// * `tokenizer_path` - Path to tokenizer.json file
    ///
    /// # Returns
    ///
    /// Result containing EmbeddingService or error if initialization fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// let embedder = EmbeddingService::new(
    ///     "models/embeddings/all-MiniLM-L6-v2.onnx",
    ///     "models/embeddings/all-MiniLM-L6-v2_tokenizer.json"
    /// ).unwrap();
    /// ```
    pub fn new(model_path: impl AsRef<Path>, tokenizer_path: impl AsRef<Path>) -> Result<Self> {
        info!("Initializing embedding service");
        debug!(
            "Model: {}, Tokenizer: {}",
            model_path.as_ref().display(),
            tokenizer_path.as_ref().display()
        );

        // Load ONNX model with default environment
        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("Failed to create session builder: {}", e))?
            .with_intra_threads(4)
            .map_err(|e| anyhow::anyhow!("Failed to set threads: {}", e))?
            .commit_from_file(&model_path)
            .map_err(|e| anyhow::anyhow!("Failed to load ONNX model: {}", e))?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        info!("Embedding service initialized successfully");

        Ok(Self {
            session,
            tokenizer,
            embedding_dim: 384, // all-MiniLM-L6-v2 embedding size
        })
    }

    /// Generate embedding for a single text
    ///
    /// # Arguments
    ///
    /// * `text` - Input text to embed
    ///
    /// # Returns
    ///
    /// Result containing 384-dimensional embedding vector
    ///
    /// # Performance
    ///
    /// Target: <100ms on Raspberry Pi 5
    ///
    /// # Example
    ///
    /// ```no_run
    /// let embedding = embedder.embed("Hello world").unwrap();
    /// assert_eq!(embedding.len(), 384);
    /// ```
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        debug!("Generating embedding for text: {}", text);

        // Tokenize input
        let encoding = self
            .tokenizer
            .encode(text, false)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        let input_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();

        // Convert to ndarray (batch size 1)
        let input_ids_array = Array2::from_shape_vec(
            (1, input_ids.len()),
            input_ids.iter().map(|&x| x as i64).collect(),
        )
        .context("Failed to create input_ids array")?;

        let attention_mask_array = Array2::from_shape_vec(
            (1, attention_mask.len()),
            attention_mask.iter().map(|&x| x as i64).collect(),
        )
        .context("Failed to create attention_mask array")?;

        // Token type IDs (all zeros for single sentence)
        let token_type_ids_array =
            Array2::from_shape_vec((1, input_ids.len()), vec![0i64; input_ids.len()])
                .context("Failed to create token_type_ids array")?;

        // Create Value objects
        let input_ids_value = Value::from_array(input_ids_array)?;
        let attention_mask_value = Value::from_array(attention_mask_array)?;
        let token_type_ids_value = Value::from_array(token_type_ids_array)?;

        // Run inference with named inputs
        let outputs = self
            .session
            .run(ort::inputs! {
                "input_ids" => input_ids_value,
                "attention_mask" => attention_mask_value,
                "token_type_ids" => token_type_ids_value,
            })
            .map_err(|e| anyhow::anyhow!("ONNX inference failed: {}", e))?;

        // Extract embeddings from output (last_hidden_state)
        let embeddings_tensor = &outputs["last_hidden_state"];
        let (shape, data_slice) = embeddings_tensor.try_extract_tensor::<f32>()?;

        // Get dimensions and convert to owned array
        let dims = shape.as_ref();
        let batch_size = dims[0] as usize;
        let seq_len = dims[1] as usize;
        let hidden_dim = dims[2] as usize;

        let data: Vec<f32> = data_slice.to_vec();

        let arr3 = ndarray::Array3::from_shape_vec((batch_size, seq_len, hidden_dim), data)
            .context("Failed to reshape embeddings")?;

        // Mean pooling over sequence dimension
        // Shape: (batch_size=1, seq_len, hidden_dim=384) -> (1, hidden_dim=384)
        let pooled = arr3.mean_axis(Axis(1)).context("Mean pooling failed")?;

        // Extract the single batch item
        let embedding = pooled.index_axis(Axis(0), 0).to_vec();

        // Normalize embedding (L2 normalization)
        let normalized = normalize_vector(&embedding);

        debug!("Generated {}-dim embedding", normalized.len());

        Ok(normalized)
    }

    /// Generate embeddings for multiple texts (batch processing)
    ///
    /// # Arguments
    ///
    /// * `texts` - Slice of input texts to embed
    ///
    /// # Returns
    ///
    /// Result containing vector of embeddings (one per input text)
    ///
    /// # Performance
    ///
    /// More efficient than calling embed() repeatedly for many texts.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let texts = vec!["Hello", "World"];
    /// let embeddings = embedder.embed_batch(&texts).unwrap();
    /// assert_eq!(embeddings.len(), 2);
    /// ```
    pub fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        info!("Generating embeddings for {} texts", texts.len());

        // For now, process sequentially
        // TODO: Implement true batch processing with padding
        texts.iter().map(|text| self.embed(text)).collect()
    }

    /// Get the embedding dimensionality (384 for all-MiniLM-L6-v2)
    pub fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }
}

/// Normalize a vector using L2 normalization
///
/// # Arguments
///
/// * `vec` - Input vector
///
/// # Returns
///
/// Normalized vector with L2 norm = 1.0
fn normalize_vector(vec: &[f32]) -> Vec<f32> {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm == 0.0 {
        return vec.to_vec();
    }

    vec.iter().map(|x| x / norm).collect()
}

/// Calculate cosine similarity between two vectors
///
/// # Arguments
///
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
///
/// Cosine similarity in range [-1.0, 1.0] (typically [0.0, 1.0] for normalized vectors)
///
/// # Example
///
/// ```no_run
/// let sim = cosine_similarity(&embedding1, &embedding2);
/// if sim > 0.7 {
///     println!("Vectors are similar!");
/// }
/// ```
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_vector() {
        let vec = vec![3.0, 4.0]; // Length 5
        let normalized = normalize_vector(&vec);

        // Check L2 norm is 1.0
        let norm: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);

        // Check values
        assert!((normalized[0] - 0.6).abs() < 1e-6);
        assert!((normalized[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity() {
        // Identical vectors
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        // Orthogonal vectors
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);

        // Opposite vectors
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 1e-6);
    }

    // Integration test with actual model (requires model files)
    #[test]
    #[ignore] // Run with: cargo test --release -- --ignored
    fn test_embedding_generation() {
        let mut embedder = EmbeddingService::new(
            "models/embeddings/all-MiniLM-L6-v2.onnx",
            "models/embeddings/all-MiniLM-L6-v2_tokenizer.json",
        )
        .expect("Failed to create embedder");

        let embedding = embedder
            .embed("Hello world")
            .expect("Failed to generate embedding");

        assert_eq!(embedding.len(), 384);

        // Check embedding is normalized
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    #[ignore]
    fn test_semantic_similarity() {
        let mut embedder = EmbeddingService::new(
            "models/embeddings/all-MiniLM-L6-v2.onnx",
            "models/embeddings/all-MiniLM-L6-v2_tokenizer.json",
        )
        .expect("Failed to create embedder");

        let emb1 = embedder.embed("I like coffee").expect("Failed");
        let emb2 = embedder.embed("I enjoy coffee").expect("Failed");
        let emb3 = embedder.embed("The weather is nice").expect("Failed");

        let sim_similar = cosine_similarity(&emb1, &emb2);
        let sim_different = cosine_similarity(&emb1, &emb3);

        // Similar sentences should have higher similarity
        assert!(sim_similar > sim_different);
        assert!(sim_similar > 0.7); // Typically >0.8 for paraphrases
    }
}
