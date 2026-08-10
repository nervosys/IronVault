# RAG and Rule-Based Systems

IronVault provides comprehensive support for **Retrieval-Augmented Generation (RAG)** and **rule-based AI systems**, including document stores, knowledge bases, rule engines, caching, and database abstractions.

## Table of Contents

- [Overview](#overview)
- [Document Store](#document-store)
- [Knowledge Base](#knowledge-base)
- [Rule Engine](#rule-engine)
- [Retrieval Cache](#retrieval-cache)
- [Database Abstraction](#database-abstraction)
- [Complete Examples](#complete-examples)
- [API Reference](#api-reference)

## Overview

The RAG module provides:

- **Document Store**: Manage documents with embeddings and metadata
- **Knowledge Base**: Text chunking, semantic search, and retrieval
- **Rule Engine**: Business logic and decision-making systems
- **Retrieval Cache**: Performance optimization for repeated queries
- **Database Abstraction**: Generic interface for data persistence

## Document Store

The `DocumentStore` manages documents with vector embeddings for semantic search.

### Creating a Document Store

```rust
use ironvault::rag::{DocumentStore, Document};
use std::collections::HashMap;

let mut store = DocumentStore::new();
```

### Adding Documents

```rust
let doc = Document {
    id: "doc1".to_string(),
    content: "Machine learning is a subset of artificial intelligence.".to_string(),
    metadata: {
        let mut m = HashMap::new();
        m.insert("category".to_string(), "AI".to_string());
        m.insert("author".to_string(), "Research Team".to_string());
        m
    },
    embedding: Some(vec![0.1, 0.2, 0.3, 0.4]), // Your embedding vector
    chunk_info: None,
};

store.add_document(doc)?;
```

### Similarity Search

```rust
let query_embedding = vec![0.15, 0.18, 0.32, 0.38];
let top_k = 5;

let results = store.search_similar(&query_embedding, top_k);

for (doc_id, similarity_score) in results {
    println!("Document: {} (similarity: {:.4})", doc_id, similarity_score);
    if let Some(doc) = store.get_document(&doc_id) {
        println!("Content: {}", doc.content);
    }
}
```

### Document Operations

```rust
// Get document by ID
let doc = store.get_document("doc1");

// Get all documents
let all_docs = store.get_all_documents();

// Delete document
store.delete_document("doc1")?;

// Get count
let count = store.count();

// Clear all
store.clear();
```

## Knowledge Base

The `KnowledgeBase` provides high-level RAG functionality with text chunking and retrieval.

### Configuration

```rust
use ironvault::rag::KnowledgeBaseConfig;

let config = KnowledgeBaseConfig {
    embedding_dim: 384,         // Embedding vector dimension
    chunk_size: 512,            // Characters per chunk
    chunk_overlap: 50,          // Overlap between chunks
    max_results: 5,             // Maximum retrieval results
    similarity_threshold: 0.5,  // Minimum similarity score
};
```

### Creating a Knowledge Base

```rust
use ironvault::rag::KnowledgeBase;

let mut kb = KnowledgeBase::new("my_knowledge_base".to_string(), config);
```

### Adding Knowledge

```rust
let doc = Document {
    id: "article_1".to_string(),
    content: "Long article content here...".to_string(),
    metadata: HashMap::new(),
    embedding: Some(your_embedding_vector),
    chunk_info: None,
};

kb.add(doc)?;
```

### Text Chunking

```rust
let long_text = "Very long document that needs to be split into smaller chunks...";
let chunks = kb.chunk_text(long_text, "parent_doc_id");

for chunk in chunks {
    println!("Chunk {}: {}", 
             chunk.chunk_info.as_ref().unwrap().chunk_index,
             chunk.content);
}
```

### Retrieval

```rust
let query_embedding = vec![0.1; 384]; // Your query embedding
let top_k = 3; // Optional, uses config.max_results if None

let relevant_docs = kb.retrieve(&query_embedding, Some(top_k));

for doc in relevant_docs {
    println!("Retrieved: {}", doc.content);
}
```

## Rule Engine

The `RuleEngine` provides a flexible rule-based decision system.

### Creating Rules

```rust
use ironvault::rag::{Rule, RuleCondition, RuleAction};

let rule = Rule {
    id: "high_confidence_rule".to_string(),
    name: "High Confidence Classification".to_string(),
    conditions: {
        let mut cond = HashMap::new();
        cond.insert("confidence".to_string(), 
                   RuleCondition::GreaterThan(0.9));
        cond.insert("model_type".to_string(), 
                   RuleCondition::Equals("production".to_string()));
        cond
    },
    actions: vec![
        RuleAction::SetValue {
            key: "status".to_string(),
            value: "approved".to_string(),
        },
        RuleAction::Log {
            level: "info".to_string(),
            message: "High confidence result approved".to_string(),
        },
    ],
    priority: 10,  // Higher = executed first
    enabled: true,
};
```

### Rule Conditions

```rust
// Exact match
RuleCondition::Equals("value".to_string())

// Contains substring
RuleCondition::Contains("keyword".to_string())

// Pattern matching
RuleCondition::Matches("pattern".to_string())

// Numeric comparisons
RuleCondition::GreaterThan(0.9)
RuleCondition::LessThan(0.1)

// List membership
RuleCondition::In(vec!["option1".to_string(), "option2".to_string()])

// Custom logic
RuleCondition::Custom("custom_logic_id".to_string())
```

### Rule Actions

```rust
// Set a value
RuleAction::SetValue {
    key: "result".to_string(),
    value: "success".to_string(),
}

// Add to list
RuleAction::AddToList {
    key: "tags".to_string(),
    value: "verified".to_string(),
}

// Log message
RuleAction::Log {
    level: "info".to_string(),
    message: "Processing completed".to_string(),
}

// Call function (handled by application)
RuleAction::CallFunction {
    function: "process_result".to_string(),
    args: vec!["arg1".to_string(), "arg2".to_string()],
}

// Stop rule processing
RuleAction::Stop
```

### Using the Rule Engine

```rust
use ironvault::rag::RuleEngine;

let mut engine = RuleEngine::new();

// Add rules
engine.add_rule(rule1);
engine.add_rule(rule2);

// Set context
engine.set_context("confidence".to_string(), "0.95".to_string());
engine.set_context("model_type".to_string(), "production".to_string());

// Execute rules
let executed_rules = engine.execute()?;

// Get results
if let Some(status) = engine.get_context("status") {
    println!("Status: {}", status);
}
```

## Retrieval Cache

The `RetrievalCache` optimizes repeated queries with LRU eviction.

### Creating a Cache

```rust
use ironvault::rag::RetrievalCache;

// 10MB cache
let mut cache = RetrievalCache::new(10 * 1024 * 1024);
```

### Caching Results

```rust
let query = "What is machine learning?";
let results = vec![doc1, doc2, doc3]; // Retrieval results

cache.cache_results(query, results)?;
```

### Cache Lookup

```rust
if let Some(cached_results) = cache.get_cached(query) {
    println!("Cache hit! Retrieved {} documents", cached_results.len());
} else {
    println!("Cache miss - performing retrieval...");
    // Perform actual retrieval
}
```

### Cache Statistics

```rust
let stats = cache.stats();
println!("Cache entries: {}", stats.entries);
println!("Cache size: {} / {} bytes", stats.size_bytes, stats.max_size_bytes);
println!("Hit rate: {:.2}%", stats.hit_rate * 100.0);
```

### Clearing Cache

```rust
cache.clear();
```

## Database Abstraction

Generic database interface with in-memory implementation.

### In-Memory Database

```rust
use ironvault::rag::{InMemoryDatabase, Database};

let mut db = InMemoryDatabase::new();

// Create table
db.create_table("users".to_string());

// Insert data
let mut user = HashMap::new();
user.insert("id".to_string(), "1".to_string());
user.insert("name".to_string(), "Alice".to_string());
user.insert("email".to_string(), "alice@example.com".to_string());

db.insert("users", user)?;
```

### Querying

```rust
// Get all records
let all_users = db.query("users")?;

// Query with WHERE clause
let results = db.query("users WHERE id=1")?;

for row in results {
    println!("Name: {}", row.get("name").unwrap());
}
```

### Updating and Deleting

```rust
// Update
let mut updates = HashMap::new();
updates.insert("email".to_string(), "alice.new@example.com".to_string());
db.update("users", "1", updates)?;

// Delete
db.delete("users", "1")?;
```

## Complete Examples

### RAG Pipeline

```rust
use ironvault::rag::*;

fn rag_pipeline(query: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // 1. Initialize components
    let config = KnowledgeBaseConfig::default();
    let mut kb = KnowledgeBase::new("docs".to_string(), config);
    let mut cache = RetrievalCache::new(5 * 1024 * 1024);
    
    // 2. Check cache
    if let Some(cached) = cache.get_cached(query) {
        return Ok(cached.iter().map(|d| d.content.clone()).collect());
    }
    
    // 3. Generate query embedding (you would use actual embedding model)
    let query_embedding = generate_embedding(query);
    
    // 4. Retrieve relevant documents
    let docs = kb.retrieve(&query_embedding, Some(5));
    
    // 5. Cache results
    cache.cache_results(query, docs.clone())?;
    
    // 6. Extract content
    Ok(docs.iter().map(|d| d.content.clone()).collect())
}
```

### Rule-Based Routing

```rust
fn route_request(request_type: &str, confidence: f64) -> String {
    let mut engine = RuleEngine::new();
    
    // High confidence rule
    let high_conf = Rule {
        id: "high_conf".to_string(),
        name: "High Confidence".to_string(),
        conditions: {
            let mut c = HashMap::new();
            c.insert("confidence".to_string(), RuleCondition::GreaterThan(0.9));
            c
        },
        actions: vec![
            RuleAction::SetValue {
                key: "route".to_string(),
                value: "fast_path".to_string(),
            }
        ],
        priority: 10,
        enabled: true,
    };
    
    // Type-based routing
    let type_rule = Rule {
        id: "type_route".to_string(),
        name: "Type Routing".to_string(),
        conditions: {
            let mut c = HashMap::new();
            c.insert("type".to_string(), RuleCondition::Equals("llm".to_string()));
            c
        },
        actions: vec![
            RuleAction::SetValue {
                key: "route".to_string(),
                value: "llm_service".to_string(),
            }
        ],
        priority: 5,
        enabled: true,
    };
    
    engine.add_rule(high_conf);
    engine.add_rule(type_rule);
    
    engine.set_context("confidence".to_string(), confidence.to_string());
    engine.set_context("type".to_string(), request_type.to_string());
    
    engine.execute().unwrap();
    
    engine.get_context("route")
        .cloned()
        .unwrap_or_else(|| "default".to_string())
}
```

## API Reference

### Document

```rust
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub embedding: Option<Vec<f32>>,
    pub chunk_info: Option<ChunkInfo>,
}
```

### DocumentStore

```rust
impl DocumentStore {
    pub fn new() -> Self;
    pub fn add_document(&mut self, doc: Document) -> Result<()>;
    pub fn get_document(&self, id: &str) -> Option<&Document>;
    pub fn search_similar(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)>;
    pub fn delete_document(&mut self, id: &str) -> Result<()>;
    pub fn count(&self) -> usize;
    pub fn clear(&mut self);
}
```

### KnowledgeBase

```rust
impl KnowledgeBase {
    pub fn new(name: String, config: KnowledgeBaseConfig) -> Self;
    pub fn add(&mut self, doc: Document) -> Result<()>;
    pub fn retrieve(&self, query_embedding: &[f32], top_k: Option<usize>) -> Vec<Document>;
    pub fn chunk_text(&self, text: &str, doc_id: &str) -> Vec<Document>;
}
```

### RuleEngine

```rust
impl RuleEngine {
    pub fn new() -> Self;
    pub fn add_rule(&mut self, rule: Rule);
    pub fn set_context(&mut self, key: String, value: String);
    pub fn get_context(&self, key: &str) -> Option<&String>;
    pub fn execute(&mut self) -> Result<Vec<String>>;
    pub fn clear_rules(&mut self);
}
```

### RetrievalCache

```rust
impl RetrievalCache {
    pub fn new(max_size: usize) -> Self;
    pub fn cache_results(&mut self, query: &str, results: Vec<Document>) -> Result<()>;
    pub fn get_cached(&mut self, query: &str) -> Option<Vec<Document>>;
    pub fn clear(&mut self);
    pub fn stats(&self) -> CacheStats;
}
```

### Database Trait

```rust
pub trait Database {
    fn query(&self, query: &str) -> Result<Vec<HashMap<String, String>>>;
    fn insert(&mut self, table: &str, data: HashMap<String, String>) -> Result<()>;
    fn update(&mut self, table: &str, id: &str, data: HashMap<String, String>) -> Result<()>;
    fn delete(&mut self, table: &str, id: &str) -> Result<()>;
}
```

## Performance Considerations

1. **Embedding Dimension**: Larger dimensions provide more semantic information but increase memory usage and search time
2. **Chunk Size**: Balance between context preservation and granularity
3. **Cache Size**: Monitor cache hit rate and adjust size accordingly
4. **Rule Priority**: Order rules by priority to optimize execution
5. **Similarity Threshold**: Higher thresholds reduce false positives but may miss relevant results

## Best Practices

1. **Normalize Embeddings**: Ensure embeddings are normalized for accurate cosine similarity
2. **Monitor Cache**: Track hit rates and eviction patterns
3. **Rule Testing**: Test rules individually before combining
4. **Chunking Strategy**: Adjust chunk size and overlap based on your content
5. **Error Handling**: Always handle database and rule execution errors
6. **Metadata**: Use document metadata for filtering and organization
7. **Batch Operations**: Add multiple documents at once for better performance

## Integration Examples

### With External Vector DB

```rust
// Implement Database trait for your vector DB
struct PineconeDB {
    client: PineconeClient,
}

impl Database for PineconeDB {
    fn query(&self, query: &str) -> Result<Vec<HashMap<String, String>>> {
        // Forward to Pinecone
        self.client.query(query)
    }
    // ... other methods
}
```

### With Embedding Models

```rust
fn embed_text(text: &str) -> Vec<f32> {
    // Use sentence-transformers, OpenAI, or other embedding models
    // This is a placeholder
    vec![0.0; 384]
}

let doc = Document {
    id: "doc1".to_string(),
    content: text.to_string(),
    metadata: HashMap::new(),
    embedding: Some(embed_text(text)),
    chunk_info: None,
};
```

## See Also

- [Utilities Guide](UTILITIES.md)
- [Security Features](https://github.com/nervosys/IronVault/blob/master/SECURITY.md)
- [API Documentation](https://docs.rs/ironvault)
