# RAG & Rule-Based Systems - Quick Reference

## Import

```rust
use ironvault::rag::*;
use std::collections::HashMap;
```

## Document Store

### Create & Add Documents
```rust
let mut store = DocumentStore::new();

let doc = Document {
    id: "doc1".to_string(),
    content: "Your text here".to_string(),
    metadata: HashMap::new(),
    embedding: Some(vec![0.1, 0.2, 0.3]),
    chunk_info: None,
};

store.add_document(doc)?;
```

### Search Similar
```rust
let query_emb = vec![0.15, 0.18, 0.32];
let results = store.search_similar(&query_emb, 5);

for (doc_id, score) in results {
    println!("{}: {:.4}", doc_id, score);
}
```

### Operations
```rust
store.get_document("doc1");       // Get by ID
store.delete_document("doc1")?;   // Delete
store.count();                     // Count
store.clear();                     // Clear all
```

## Knowledge Base

### Create
```rust
let config = KnowledgeBaseConfig {
    embedding_dim: 384,
    chunk_size: 512,
    chunk_overlap: 50,
    max_results: 5,
    similarity_threshold: 0.5,
};

let mut kb = KnowledgeBase::new("my_kb".to_string(), config);
```

### Add & Retrieve
```rust
kb.add(document)?;

let results = kb.retrieve(&query_embedding, Some(3));
```

### Chunking
```rust
let chunks = kb.chunk_text("Long text...", "doc_id");
```

## Rule Engine

### Create Rules
```rust
let rule = Rule {
    id: "rule1".to_string(),
    name: "My Rule".to_string(),
    conditions: {
        let mut c = HashMap::new();
        c.insert("key".to_string(), 
                RuleCondition::GreaterThan(0.9));
        c
    },
    actions: vec![
        RuleAction::SetValue {
            key: "result".to_string(),
            value: "success".to_string(),
        }
    ],
    priority: 10,
    enabled: true,
};
```

### Execute
```rust
let mut engine = RuleEngine::new();
engine.add_rule(rule);
engine.set_context("key".to_string(), "0.95".to_string());

let executed = engine.execute()?;
let result = engine.get_context("result");
```

### Conditions
```rust
RuleCondition::Equals("value".to_string())
RuleCondition::Contains("keyword".to_string())
RuleCondition::GreaterThan(0.9)
RuleCondition::LessThan(0.1)
RuleCondition::In(vec!["a".to_string(), "b".to_string()])
```

### Actions
```rust
RuleAction::SetValue { key, value }
RuleAction::AddToList { key, value }
RuleAction::Log { level, message }
RuleAction::Stop
```

## Retrieval Cache

### Basic Usage
```rust
let mut cache = RetrievalCache::new(10 * 1024 * 1024); // 10MB

// Cache
cache.cache_results("query", vec![doc1, doc2])?;

// Retrieve
if let Some(results) = cache.get_cached("query") {
    // Cache hit!
}

// Stats
let stats = cache.stats();
println!("Hit rate: {:.1}%", stats.hit_rate * 100.0);
```

## Database

### Create & Use
```rust
let mut db = InMemoryDatabase::new();
db.create_table("users".to_string());

// Insert
let mut data = HashMap::new();
data.insert("id".to_string(), "1".to_string());
data.insert("name".to_string(), "Alice".to_string());
db.insert("users", data)?;

// Query
let all = db.query("users")?;
let filtered = db.query("users WHERE id=1")?;

// Update
let mut updates = HashMap::new();
updates.insert("name".to_string(), "Bob".to_string());
db.update("users", "1", updates)?;

// Delete
db.delete("users", "1")?;
```

## Complete RAG Pipeline

```rust
fn rag_query(query: &str) -> Result<String> {
    // 1. Setup
    let config = KnowledgeBaseConfig::default();
    let mut kb = KnowledgeBase::new("kb".to_string(), config);
    let mut cache = RetrievalCache::new(5 * 1024 * 1024);
    
    // 2. Check cache
    if let Some(cached) = cache.get_cached(query) {
        return Ok(format_response(cached));
    }
    
    // 3. Generate embedding
    let query_emb = embed_text(query);
    
    // 4. Retrieve context
    let docs = kb.retrieve(&query_emb, Some(3));
    
    // 5. Cache results
    cache.cache_results(query, docs.clone())?;
    
    // 6. Generate response
    Ok(format_response(docs))
}
```

## Common Patterns

### RAG with Rules
```rust
let mut engine = RuleEngine::new();

// Add confidence threshold rule
let rule = Rule {
    id: "confidence".to_string(),
    name: "Confidence Check".to_string(),
    conditions: {
        let mut c = HashMap::new();
        c.insert("score".to_string(), 
                RuleCondition::LessThan(0.5));
        c
    },
    actions: vec![
        RuleAction::SetValue {
            key: "fallback".to_string(),
            value: "true".to_string(),
        }
    ],
    priority: 10,
    enabled: true,
};

engine.add_rule(rule);

// After retrieval, check quality
for (doc_id, score) in results {
    engine.set_context("score".to_string(), score.to_string());
    engine.execute()?;
    
    if engine.get_context("fallback") == Some(&"true".to_string()) {
        // Use fallback strategy
    }
}
```

### Multi-KB Search
```rust
let kb1 = KnowledgeBase::new("technical".to_string(), config.clone());
let kb2 = KnowledgeBase::new("general".to_string(), config);

let results1 = kb1.retrieve(&query_emb, Some(2));
let results2 = kb2.retrieve(&query_emb, Some(2));

let combined: Vec<_> = results1.into_iter()
    .chain(results2.into_iter())
    .collect();
```

### Cache-Aware Retrieval
```rust
fn cached_retrieve(
    query: &str,
    kb: &KnowledgeBase,
    cache: &mut RetrievalCache,
) -> Vec<Document> {
    // Try cache first
    if let Some(cached) = cache.get_cached(query) {
        return cached;
    }
    
    // Fallback to KB
    let query_emb = embed_text(query);
    let results = kb.retrieve(&query_emb, Some(5));
    
    // Cache for next time
    cache.cache_results(query, results.clone()).ok();
    
    results
}
```

## Performance Tips

1. **Normalize embeddings** for accurate cosine similarity
2. **Cache frequently accessed** documents and queries
3. **Use appropriate chunk sizes** (smaller = more precise, larger = more context)
4. **Monitor cache hit rates** and adjust size
5. **Batch document additions** when possible
6. **Set rule priorities** to optimize execution
7. **Use metadata filtering** before semantic search

## See Also

- [Full Documentation](RAG.md)
- [Examples](https://github.com/nervosys/IronVault/blob/master/examples/rag_demo.rs)
- [Tests](https://github.com/nervosys/IronVault/blob/master/tests/rag_tests.rs)
