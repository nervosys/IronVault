//! Demonstration of RAG and Rule-Based System Features
//!
//! This example shows how to use:
//! - Document stores for RAG systems
//! - Knowledge bases with embedding search
//! - Rule engines for business logic
//! - In-memory databases
//! - Retrieval caching

use ironvault::rag::*;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IronVault RAG and Rule-Based Systems Demo ===\n");

    // PART 1: Document Store for RAG
    println!("📚 PART 1: Document Store for RAG");
    println!("----------------------------------");
    demo_document_store()?;

    // PART 2: Knowledge Base with Semantic Search
    println!("\n🔍 PART 2: Knowledge Base with Semantic Search");
    println!("-----------------------------------------------");
    demo_knowledge_base()?;

    // PART 3: Rule Engine
    println!("\n⚙️  PART 3: Rule-Based System");
    println!("---------------------------");
    demo_rule_engine()?;

    // PART 4: In-Memory Database
    println!("\n💾 PART 4: In-Memory Database");
    println!("----------------------------");
    demo_database()?;

    // PART 5: Retrieval Cache
    println!("\n🚀 PART 5: Retrieval Cache");
    println!("-------------------------");
    demo_retrieval_cache()?;

    // PART 6: Complete RAG Pipeline
    println!("\n🎯 PART 6: Complete RAG Pipeline");
    println!("--------------------------------");
    demo_complete_rag_pipeline()?;

    println!("\n✅ All demonstrations completed successfully!");
    Ok(())
}

fn demo_document_store() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = DocumentStore::new();

    // Add documents
    println!("Adding documents to store...");

    let doc1 = Document {
        id: "ai_basics".to_string(),
        content: "Artificial Intelligence is the simulation of human intelligence by machines."
            .to_string(),
        metadata: {
            let mut m = HashMap::new();
            m.insert("category".to_string(), "AI".to_string());
            m.insert("topic".to_string(), "Introduction".to_string());
            m
        },
        embedding: Some(vec![0.9, 0.1, 0.05, 0.02]),
        chunk_info: None,
    };

    let doc2 = Document {
        id: "ml_basics".to_string(),
        content:
            "Machine Learning is a subset of AI that learns from data without explicit programming."
                .to_string(),
        metadata: {
            let mut m = HashMap::new();
            m.insert("category".to_string(), "ML".to_string());
            m.insert("topic".to_string(), "Introduction".to_string());
            m
        },
        embedding: Some(vec![0.85, 0.15, 0.08, 0.03]),
        chunk_info: None,
    };

    let doc3 = Document {
        id: "deep_learning".to_string(),
        content:
            "Deep Learning uses neural networks with multiple layers to learn complex patterns."
                .to_string(),
        metadata: {
            let mut m = HashMap::new();
            m.insert("category".to_string(), "DL".to_string());
            m.insert("topic".to_string(), "Neural Networks".to_string());
            m
        },
        embedding: Some(vec![0.7, 0.25, 0.3, 0.1]),
        chunk_info: None,
    };

    store.add_document(doc1)?;
    store.add_document(doc2)?;
    store.add_document(doc3)?;

    println!("✓ Added {} documents", store.count());

    // Search for similar documents
    println!("\nSearching for documents similar to AI query...");
    let query_embedding = vec![0.92, 0.08, 0.03, 0.01];
    let results = store.search_similar(&query_embedding, 2);

    for (i, (doc_id, score)) in results.iter().enumerate() {
        println!(
            "  {}. Document: {} (similarity: {:.4})",
            i + 1,
            doc_id,
            score
        );
        if let Some(doc) = store.get_document(doc_id) {
            println!(
                "     Content: {}",
                &doc.content[..60.min(doc.content.len())]
            );
        }
    }

    Ok(())
}

fn demo_knowledge_base() -> Result<(), Box<dyn std::error::Error>> {
    let config = KnowledgeBaseConfig {
        embedding_dim: 4,
        chunk_size: 50,
        chunk_overlap: 10,
        max_results: 3,
        similarity_threshold: 0.3,
    };

    let mut kb = KnowledgeBase::new("ai_knowledge".to_string(), config);

    println!("Knowledge Base: {}", kb.name);
    println!(
        "Configuration: {} dim embeddings, {} char chunks",
        kb.config.embedding_dim, kb.config.chunk_size
    );

    // Add knowledge
    println!("\nAdding knowledge articles...");

    let articles = vec![
        ("transformers", "Transformers revolutionized NLP with attention mechanisms that process sequences in parallel."),
        ("rag", "Retrieval-Augmented Generation combines information retrieval with language generation for better responses."),
        ("embeddings", "Embeddings are dense vector representations that capture semantic meaning of text."),
    ];

    for (id, content) in articles {
        let doc = Document {
            id: id.to_string(),
            content: content.to_string(),
            metadata: HashMap::new(),
            embedding: Some(generate_mock_embedding(content)),
            chunk_info: None,
        };
        kb.add(doc)?;
        println!("  ✓ Added article: {}", id);
    }

    // Chunking demonstration
    println!("\nDemonstrating text chunking...");
    let long_text = "This is a longer document that needs to be split into smaller chunks for more efficient processing and retrieval in RAG systems. Each chunk will have some overlap to maintain context.";
    let chunks = kb.chunk_text(long_text, "long_doc");

    println!("  Split into {} chunks:", chunks.len());
    for chunk in &chunks {
        let preview = chunk.content.chars().take(40).collect::<String>();
        println!(
            "    - Chunk {}: {}...",
            chunk.chunk_info.as_ref().unwrap().chunk_index,
            preview
        );
    }

    // Retrieval
    println!("\nRetrieving relevant knowledge for query about 'generation'...");
    let query_emb = generate_mock_embedding("generation");
    let results = kb.retrieve(&query_emb, Some(2));

    for (i, doc) in results.iter().enumerate() {
        println!(
            "  {}. {}: {}",
            i + 1,
            doc.id,
            &doc.content[..50.min(doc.content.len())]
        );
    }

    Ok(())
}

fn demo_rule_engine() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = RuleEngine::new();

    println!("Creating rule-based system...");

    // Rule 1: High confidence classification
    let rule1 = Rule {
        id: "high_confidence".to_string(),
        name: "High Confidence Rule".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert("confidence".to_string(), RuleCondition::GreaterThan(0.9));
            cond
        },
        actions: vec![
            RuleAction::SetValue {
                key: "classification".to_string(),
                value: "accepted".to_string(),
            },
            RuleAction::Log {
                level: "info".to_string(),
                message: "High confidence classification".to_string(),
            },
        ],
        priority: 10,
        enabled: true,
    };

    // Rule 2: Model type routing
    let rule2 = Rule {
        id: "model_routing".to_string(),
        name: "Model Type Routing".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert(
                "model_type".to_string(),
                RuleCondition::Equals("llm".to_string()),
            );
            cond
        },
        actions: vec![
            RuleAction::SetValue {
                key: "endpoint".to_string(),
                value: "llm_service".to_string(),
            },
            RuleAction::SetValue {
                key: "max_tokens".to_string(),
                value: "2048".to_string(),
            },
        ],
        priority: 5,
        enabled: true,
    };

    // Rule 3: Error handling
    let rule3 = Rule {
        id: "error_handler".to_string(),
        name: "Error Handler".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert(
                "status".to_string(),
                RuleCondition::Contains("error".to_string()),
            );
            cond
        },
        actions: vec![
            RuleAction::SetValue {
                key: "retry".to_string(),
                value: "true".to_string(),
            },
            RuleAction::Stop,
        ],
        priority: 20, // Highest priority
        enabled: true,
    };

    engine.add_rule(rule1);
    engine.add_rule(rule2);
    engine.add_rule(rule3);

    println!("✓ Added {} rules", engine.get_rules().len());

    // Scenario 1: Normal operation
    println!("\nScenario 1: Normal operation");
    engine.set_context("confidence".to_string(), "0.95".to_string());
    engine.set_context("model_type".to_string(), "llm".to_string());

    let executed = engine.execute()?;
    println!("  Executed rules: {:?}", executed);
    println!(
        "  Classification: {:?}",
        engine.get_context("classification")
    );
    println!("  Endpoint: {:?}", engine.get_context("endpoint"));

    // Scenario 2: Error handling
    println!("\nScenario 2: Error condition");
    engine.set_context("status".to_string(), "error occurred".to_string());

    let executed = engine.execute()?;
    println!("  Executed rules: {:?}", executed);
    println!("  Retry flag: {:?}", engine.get_context("retry"));

    Ok(())
}

fn demo_database() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = InMemoryDatabase::new();

    println!("Creating in-memory database...");

    // Create tables
    db.create_table("models".to_string());
    db.create_table("deployments".to_string());

    println!("✓ Created tables: models, deployments");

    // Insert model records
    println!("\nInserting model records...");

    let models = vec![
        ("1", "GPT-4", "OpenAI", "active"),
        ("2", "LLaMA-2", "Meta", "active"),
        ("3", "BERT", "Google", "archived"),
    ];

    for (id, name, provider, status) in models {
        let mut data = HashMap::new();
        data.insert("id".to_string(), id.to_string());
        data.insert("name".to_string(), name.to_string());
        data.insert("provider".to_string(), provider.to_string());
        data.insert("status".to_string(), status.to_string());

        db.insert("models", data)?;
        println!("  ✓ Inserted: {} ({})", name, provider);
    }

    // Query all models
    println!("\nQuerying all models:");
    let all_models = db.query("models")?;
    println!("  Found {} models", all_models.len());

    // Query with filter
    println!("\nQuerying active models:");
    let active_models = db.query("models WHERE status=active")?;
    for model in active_models {
        println!(
            "  - {} by {}",
            model.get("name").unwrap(),
            model.get("provider").unwrap()
        );
    }

    // Update record
    println!("\nUpdating model status...");
    let mut update = HashMap::new();
    update.insert("status".to_string(), "deprecated".to_string());
    db.update("models", "3", update)?;
    println!("  ✓ Updated BERT status");

    // Delete record
    println!("\nDeleting model...");
    db.delete("models", "3")?;
    println!("  ✓ Deleted model with id=3");

    let remaining = db.query("models")?;
    println!("  Remaining models: {}", remaining.len());

    Ok(())
}

fn demo_retrieval_cache() -> Result<(), Box<dyn std::error::Error>> {
    let mut cache = RetrievalCache::new(10240); // 10KB cache

    println!("Creating retrieval cache (10KB)...");

    // Simulate retrieval results
    let queries = vec![
        "What is machine learning?",
        "How do transformers work?",
        "Explain neural networks",
    ];

    println!("\nCaching retrieval results...");
    for query in &queries {
        let docs = vec![Document {
            id: format!("result_for_{}", query.split_whitespace().next().unwrap()),
            content: format!("Answer to: {}", query),
            metadata: HashMap::new(),
            embedding: None,
            chunk_info: None,
        }];
        cache.cache_results(query, docs)?;
        println!("  ✓ Cached: {}", query);
    }

    // Test cache hits
    println!("\nTesting cache retrieval...");
    for query in &queries {
        if let Some(results) = cache.get_cached(query) {
            println!("  ✓ Cache HIT for: {}", query);
            println!("    Retrieved: {}", results[0].content);
        } else {
            println!("  ✗ Cache MISS for: {}", query);
        }
    }

    // Cache statistics
    let stats = cache.stats();
    println!("\nCache Statistics:");
    println!("  Entries: {}", stats.entries);
    println!(
        "  Size: {} / {} bytes",
        stats.size_bytes, stats.max_size_bytes
    );
    println!(
        "  Usage: {:.1}%",
        (stats.size_bytes as f64 / stats.max_size_bytes as f64) * 100.0
    );

    Ok(())
}

fn demo_complete_rag_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    println!("Building complete RAG pipeline...");

    // Step 1: Knowledge Base
    let config = KnowledgeBaseConfig {
        embedding_dim: 4,
        chunk_size: 100,
        chunk_overlap: 20,
        max_results: 3,
        similarity_threshold: 0.2,
    };
    let mut kb = KnowledgeBase::new("production_kb".to_string(), config);

    // Step 2: Retrieval Cache
    let mut cache = RetrievalCache::new(5120);

    // Step 3: Rule Engine for query processing
    let mut rules = RuleEngine::new();

    let query_rule = Rule {
        id: "query_enhancement".to_string(),
        name: "Query Enhancement".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert("query_length".to_string(), RuleCondition::LessThan(10.0));
            cond
        },
        actions: vec![RuleAction::SetValue {
            key: "expand_query".to_string(),
            value: "true".to_string(),
        }],
        priority: 10,
        enabled: true,
    };
    rules.add_rule(query_rule);

    // Add knowledge
    println!("\n1. Populating knowledge base...");
    let knowledge_items = [
        "RAG systems combine retrieval with generation for better AI responses",
        "Vector databases store embeddings for efficient semantic search",
        "Fine-tuning adapts pre-trained models to specific tasks",
        "Prompt engineering improves model outputs through better instructions",
    ];

    for (i, content) in knowledge_items.iter().enumerate() {
        let doc = Document {
            id: format!("kb_item_{}", i),
            content: content.to_string(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("source".to_string(), "knowledge_base".to_string());
                m
            },
            embedding: Some(generate_mock_embedding(content)),
            chunk_info: None,
        };
        kb.add(doc)?;
    }
    println!("  ✓ Added {} knowledge items", kb.store.count());

    // Process query
    println!("\n2. Processing query...");
    let query = "How do RAG systems work?";
    println!("  Query: '{}'", query);

    // Check cache first
    println!("\n3. Checking cache...");
    let results = if let Some(cached) = cache.get_cached(query) {
        println!("  ✓ Cache HIT!");
        cached
    } else {
        println!("  ✗ Cache MISS - retrieving from KB...");

        // Apply rules
        rules.set_context("query_length".to_string(), query.len().to_string());
        rules.execute()?;

        // Retrieve from KB
        let query_emb = generate_mock_embedding(query);
        let retrieved = kb.retrieve(&query_emb, Some(2));

        // Cache results
        cache.cache_results(query, retrieved.clone())?;
        retrieved
    };

    println!("\n4. Retrieved context:");
    for (i, doc) in results.iter().enumerate() {
        println!("  {}. {}", i + 1, doc.content);
    }

    println!("\n5. Generating response...");
    println!("  [Simulated] Combining retrieved context with LLM generation...");
    println!("  Response: RAG systems enhance AI by retrieving relevant information");
    println!("           from a knowledge base before generating responses, ensuring");
    println!("           more accurate and contextual outputs.");

    Ok(())
}

// Helper function to generate mock embeddings based on text
fn generate_mock_embedding(text: &str) -> Vec<f32> {
    // Simple mock: use character counts for different features
    let len = text.len() as f32;
    let words = text.split_whitespace().count() as f32;
    let has_ai = if text.to_lowercase().contains("ai") {
        1.0
    } else {
        0.0
    };
    let has_data = if text.to_lowercase().contains("data") {
        1.0
    } else {
        0.0
    };

    vec![len / 100.0, words / 20.0, has_ai, has_data]
}
