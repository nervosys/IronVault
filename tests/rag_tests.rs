// Assertions in these tests compare literal constants that round-trip
// bit-for-bit and build fixed strings; the lints below are noise here.
#![allow(clippy::float_cmp)]
use ironvault::rag::*;
use std::collections::HashMap;

#[test]
fn test_document_creation() {
    let doc = Document {
        id: "test_doc".to_string(),
        content: "This is a test document".to_string(),
        metadata: HashMap::new(),
        embedding: None,
        chunk_info: None,
    };

    assert_eq!(doc.id, "test_doc");
    assert_eq!(doc.content, "This is a test document");
    assert!(doc.embedding.is_none());
}

#[test]
fn test_document_store_add_get() {
    let mut store = DocumentStore::new();

    let doc = Document {
        id: "doc1".to_string(),
        content: "Test content".to_string(),
        metadata: HashMap::new(),
        embedding: Some(vec![0.1, 0.2, 0.3]),
        chunk_info: None,
    };

    store.add_document(doc.clone()).unwrap();
    assert_eq!(store.count(), 1);

    let retrieved = store.get_document("doc1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().content, "Test content");
}

#[test]
fn test_document_store_delete() {
    let mut store = DocumentStore::new();

    let doc = Document {
        id: "doc_to_delete".to_string(),
        content: "Will be deleted".to_string(),
        metadata: HashMap::new(),
        embedding: None,
        chunk_info: None,
    };

    store.add_document(doc).unwrap();
    assert_eq!(store.count(), 1);

    store.delete_document("doc_to_delete").unwrap();
    assert_eq!(store.count(), 0);
}

#[test]
fn test_document_store_similarity_search() {
    let mut store = DocumentStore::new();

    // Add documents with embeddings
    let doc1 = Document {
        id: "doc1".to_string(),
        content: "Document 1".to_string(),
        metadata: HashMap::new(),
        embedding: Some(vec![1.0, 0.0, 0.0]),
        chunk_info: None,
    };

    let doc2 = Document {
        id: "doc2".to_string(),
        content: "Document 2".to_string(),
        metadata: HashMap::new(),
        embedding: Some(vec![0.9, 0.1, 0.0]),
        chunk_info: None,
    };

    let doc3 = Document {
        id: "doc3".to_string(),
        content: "Document 3".to_string(),
        metadata: HashMap::new(),
        embedding: Some(vec![0.0, 1.0, 0.0]),
        chunk_info: None,
    };

    store.add_document(doc1).unwrap();
    store.add_document(doc2).unwrap();
    store.add_document(doc3).unwrap();

    // Query with vector similar to doc1
    let query = vec![1.0, 0.0, 0.0];
    let results = store.search_similar(&query, 2);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, "doc1"); // Most similar
    assert!(results[0].1 > results[1].1); // Higher similarity score
}

#[test]
fn test_knowledge_base_creation() {
    let config = KnowledgeBaseConfig::default();
    let kb = KnowledgeBase::new("test_kb".to_string(), config.clone());

    assert_eq!(kb.name, "test_kb");
    assert_eq!(kb.config.embedding_dim, 384);
    assert_eq!(kb.store.count(), 0);
}

#[test]
fn test_knowledge_base_add_retrieve() {
    let config = KnowledgeBaseConfig::default();
    let mut kb = KnowledgeBase::new("test_kb".to_string(), config);

    let doc = Document {
        id: "kb_doc1".to_string(),
        content: "Knowledge base document".to_string(),
        metadata: HashMap::new(),
        embedding: Some(vec![1.0; 384]), // Match embedding_dim
        chunk_info: None,
    };

    kb.add(doc.clone()).unwrap();
    assert_eq!(kb.store.count(), 1);

    // Retrieve with similar embedding
    let query_embedding = vec![0.99; 384];
    let results = kb.retrieve(&query_embedding, Some(1));
    assert_eq!(results.len(), 1);
}

#[test]
fn test_text_chunking() {
    let config = KnowledgeBaseConfig {
        chunk_size: 20,
        chunk_overlap: 5,
        ..Default::default()
    };
    let kb = KnowledgeBase::new("chunk_test".to_string(), config);

    let text = "This is a longer text that needs to be split into multiple chunks for processing.";
    let chunks = kb.chunk_text(text, "parent_doc");

    assert!(chunks.len() > 1);

    // Check chunk info
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.chunk_info.as_ref().unwrap().chunk_index, i);
        assert_eq!(
            chunk.chunk_info.as_ref().unwrap().total_chunks,
            chunks.len()
        );
        assert!(chunk.id.contains("parent_doc"));
    }
}

#[test]
fn test_rule_engine_basic() {
    let mut engine = RuleEngine::new();

    let rule = Rule {
        id: "rule1".to_string(),
        name: "Test Rule".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert(
                "status".to_string(),
                RuleCondition::Equals("active".to_string()),
            );
            cond
        },
        actions: vec![RuleAction::SetValue {
            key: "result".to_string(),
            value: "success".to_string(),
        }],
        priority: 10,
        enabled: true,
    };

    engine.add_rule(rule);
    engine.set_context("status".to_string(), "active".to_string());

    let executed = engine.execute().unwrap();
    assert_eq!(executed.len(), 1);
    assert_eq!(engine.get_context("result"), Some(&"success".to_string()));
}

#[test]
fn test_rule_engine_conditions() {
    let mut engine = RuleEngine::new();

    // Test Equals condition
    engine.set_context("key1".to_string(), "value1".to_string());
    let rule1 = Rule {
        id: "equals_rule".to_string(),
        name: "Equals Test".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert(
                "key1".to_string(),
                RuleCondition::Equals("value1".to_string()),
            );
            cond
        },
        actions: vec![RuleAction::SetValue {
            key: "equals_result".to_string(),
            value: "true".to_string(),
        }],
        priority: 10,
        enabled: true,
    };

    engine.add_rule(rule1);
    engine.execute().unwrap();
    assert_eq!(
        engine.get_context("equals_result"),
        Some(&"true".to_string())
    );
}

#[test]
fn test_rule_engine_contains_condition() {
    let mut engine = RuleEngine::new();

    engine.set_context("message".to_string(), "This is a test message".to_string());
    let rule = Rule {
        id: "contains_rule".to_string(),
        name: "Contains Test".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert(
                "message".to_string(),
                RuleCondition::Contains("test".to_string()),
            );
            cond
        },
        actions: vec![RuleAction::SetValue {
            key: "found".to_string(),
            value: "yes".to_string(),
        }],
        priority: 10,
        enabled: true,
    };

    engine.add_rule(rule);
    engine.execute().unwrap();
    assert_eq!(engine.get_context("found"), Some(&"yes".to_string()));
}

#[test]
fn test_rule_engine_numeric_conditions() {
    let mut engine = RuleEngine::new();

    engine.set_context("score".to_string(), "85".to_string());

    let rule = Rule {
        id: "threshold_rule".to_string(),
        name: "Threshold Test".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert("score".to_string(), RuleCondition::GreaterThan(80.0));
            cond
        },
        actions: vec![RuleAction::SetValue {
            key: "grade".to_string(),
            value: "A".to_string(),
        }],
        priority: 10,
        enabled: true,
    };

    engine.add_rule(rule);
    engine.execute().unwrap();
    assert_eq!(engine.get_context("grade"), Some(&"A".to_string()));
}

#[test]
fn test_rule_engine_priority() {
    let mut engine = RuleEngine::new();

    engine.set_context("type".to_string(), "test".to_string());

    let low_priority = Rule {
        id: "low".to_string(),
        name: "Low Priority".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert(
                "type".to_string(),
                RuleCondition::Equals("test".to_string()),
            );
            cond
        },
        actions: vec![RuleAction::SetValue {
            key: "priority_result".to_string(),
            value: "low".to_string(),
        }],
        priority: 1,
        enabled: true,
    };

    let high_priority = Rule {
        id: "high".to_string(),
        name: "High Priority".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert(
                "type".to_string(),
                RuleCondition::Equals("test".to_string()),
            );
            cond
        },
        actions: vec![RuleAction::SetValue {
            key: "priority_result".to_string(),
            value: "high".to_string(),
        }],
        priority: 10,
        enabled: true,
    };

    engine.add_rule(low_priority);
    engine.add_rule(high_priority);

    engine.execute().unwrap();
    // High priority rule should execute first and be overridden by low priority
    assert_eq!(
        engine.get_context("priority_result"),
        Some(&"low".to_string())
    );
}

#[test]
fn test_rule_engine_stop_action() {
    let mut engine = RuleEngine::new();

    engine.set_context("status".to_string(), "active".to_string());

    let stop_rule = Rule {
        id: "stop_rule".to_string(),
        name: "Stop Rule".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert(
                "status".to_string(),
                RuleCondition::Equals("active".to_string()),
            );
            cond
        },
        actions: vec![
            RuleAction::SetValue {
                key: "first".to_string(),
                value: "executed".to_string(),
            },
            RuleAction::Stop,
        ],
        priority: 20,
        enabled: true,
    };

    let second_rule = Rule {
        id: "second_rule".to_string(),
        name: "Second Rule".to_string(),
        conditions: {
            let mut cond = HashMap::new();
            cond.insert(
                "status".to_string(),
                RuleCondition::Equals("active".to_string()),
            );
            cond
        },
        actions: vec![RuleAction::SetValue {
            key: "second".to_string(),
            value: "executed".to_string(),
        }],
        priority: 10,
        enabled: true,
    };

    engine.add_rule(stop_rule);
    engine.add_rule(second_rule);

    engine.execute().unwrap();

    assert_eq!(engine.get_context("first"), Some(&"executed".to_string()));
    assert_eq!(engine.get_context("second"), None); // Should not execute due to Stop
}

#[test]
fn test_retrieval_cache() {
    let mut cache = RetrievalCache::new(10240); // 10KB cache

    let doc = Document {
        id: "cached_doc".to_string(),
        content: "Cached content".to_string(),
        metadata: HashMap::new(),
        embedding: None,
        chunk_info: None,
    };

    cache
        .cache_results("test query", vec![doc.clone()])
        .unwrap();

    let cached = cache.get_cached("test query");
    assert!(cached.is_some());
    assert_eq!(cached.unwrap()[0].id, "cached_doc");
}

#[test]
fn test_retrieval_cache_miss() {
    let mut cache = RetrievalCache::new(1024);

    let cached = cache.get_cached("nonexistent query");
    assert!(cached.is_none());
}

#[test]
fn test_retrieval_cache_eviction() {
    let mut cache = RetrievalCache::new(100); // Small cache

    // Add documents that exceed cache size
    for i in 0..10 {
        let doc = Document {
            id: format!("doc{}", i),
            content: "A".repeat(50), // 50 bytes each
            metadata: HashMap::new(),
            embedding: None,
            chunk_info: None,
        };
        cache
            .cache_results(&format!("query{}", i), vec![doc])
            .unwrap();
    }

    let stats = cache.stats();
    assert!(stats.size_bytes <= 100);
}

#[test]
fn test_in_memory_database() {
    let mut db = InMemoryDatabase::new();
    db.create_table("test_table".to_string());

    let mut data = HashMap::new();
    data.insert("id".to_string(), "1".to_string());
    data.insert("name".to_string(), "Test".to_string());

    db.insert("test_table", data).unwrap();

    let results = db.query("test_table").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get("name"), Some(&"Test".to_string()));
}

#[test]
fn test_database_query_with_where() {
    let mut db = InMemoryDatabase::new();
    db.create_table("users".to_string());

    let mut user1 = HashMap::new();
    user1.insert("id".to_string(), "1".to_string());
    user1.insert("name".to_string(), "Alice".to_string());

    let mut user2 = HashMap::new();
    user2.insert("id".to_string(), "2".to_string());
    user2.insert("name".to_string(), "Bob".to_string());

    db.insert("users", user1).unwrap();
    db.insert("users", user2).unwrap();

    let results = db.query("users WHERE id=1").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get("name"), Some(&"Alice".to_string()));
}

#[test]
fn test_database_update() {
    let mut db = InMemoryDatabase::new();
    db.create_table("records".to_string());

    let mut record = HashMap::new();
    record.insert("id".to_string(), "1".to_string());
    record.insert("value".to_string(), "old".to_string());

    db.insert("records", record).unwrap();

    let mut update_data = HashMap::new();
    update_data.insert("value".to_string(), "new".to_string());

    db.update("records", "1", update_data).unwrap();

    let results = db.query("records WHERE id=1").unwrap();
    assert_eq!(results[0].get("value"), Some(&"new".to_string()));
}

#[test]
fn test_database_delete() {
    let mut db = InMemoryDatabase::new();
    db.create_table("items".to_string());

    let mut item = HashMap::new();
    item.insert("id".to_string(), "1".to_string());
    item.insert("name".to_string(), "Item 1".to_string());

    db.insert("items", item).unwrap();

    let before = db.query("items").unwrap();
    assert_eq!(before.len(), 1);

    db.delete("items", "1").unwrap();

    let after = db.query("items").unwrap();
    assert_eq!(after.len(), 0);
}

#[test]
fn test_chunk_info() {
    let chunk_info = ChunkInfo {
        parent_id: Some("parent".to_string()),
        chunk_index: 0,
        total_chunks: 5,
        overlap: 10,
    };

    assert_eq!(chunk_info.parent_id, Some("parent".to_string()));
    assert_eq!(chunk_info.chunk_index, 0);
    assert_eq!(chunk_info.total_chunks, 5);
}

#[test]
fn test_knowledge_base_config_default() {
    let config = KnowledgeBaseConfig::default();

    assert_eq!(config.embedding_dim, 384);
    assert_eq!(config.chunk_size, 512);
    assert_eq!(config.chunk_overlap, 50);
    assert_eq!(config.max_results, 5);
    assert_eq!(config.similarity_threshold, 0.5);
}

#[test]
fn test_multiple_documents_in_knowledge_base() {
    let config = KnowledgeBaseConfig::default();
    let mut kb = KnowledgeBase::new("multi_doc_kb".to_string(), config);

    for i in 0..10 {
        let doc = Document {
            id: format!("doc{}", i),
            content: format!("Document content {}", i),
            metadata: HashMap::new(),
            embedding: Some(vec![i as f32 / 10.0; 384]),
            chunk_info: None,
        };
        kb.add(doc).unwrap();
    }

    assert_eq!(kb.store.count(), 10);
}

// MCP and Tools Tests

#[test]
fn test_mcp_tool_builder() {
    let tool = MCPTool::new(
        "search_tool".to_string(),
        "Search for documents".to_string(),
    )
    .add_parameter("query", "string", "Search query", true)
    .add_parameter("limit", "number", "Max results", false)
    .with_metadata("version".to_string(), "1.0".to_string());

    assert_eq!(tool.name, "search_tool");
    assert_eq!(tool.description, "Search for documents");
    assert_eq!(tool.metadata.get("version"), Some(&"1.0".to_string()));

    // Check schema has parameters
    let props = tool.input_schema.get("properties").unwrap();
    assert!(props.get("query").is_some());
    assert!(props.get("limit").is_some());
}

#[test]
fn test_tool_context_builder() {
    let ctx = ToolContext::new()
        .with_document_store("store_id_123".to_string())
        .with_knowledge_base("kb_id_456".to_string())
        .with_data("user_id".to_string(), "user123".to_string())
        .with_data("session".to_string(), "session456".to_string());

    assert_eq!(ctx.document_store, Some("store_id_123".to_string()));
    assert_eq!(ctx.knowledge_base, Some("kb_id_456".to_string()));
    assert_eq!(ctx.data.get("user_id"), Some(&"user123".to_string()));
    assert_eq!(ctx.data.get("session"), Some(&"session456".to_string()));
}

#[test]
fn test_tool_result_success() {
    let result = ToolResult::success(serde_json::json!({
        "documents": ["doc1", "doc2"],
        "count": 2
    }))
    .with_metadata("execution_time".to_string(), "50ms".to_string());

    assert!(result.success);
    assert!(result.error.is_none());
    assert_eq!(result.data.get("count").unwrap().as_u64().unwrap(), 2);
    assert_eq!(
        result.metadata.get("execution_time"),
        Some(&"50ms".to_string())
    );
}

#[test]
fn test_tool_result_failure() {
    let result = ToolResult::failure("Document not found".to_string())
        .with_metadata("error_code".to_string(), "404".to_string());

    assert!(!result.success);
    assert_eq!(result.error, Some("Document not found".to_string()));
    assert_eq!(result.metadata.get("error_code"), Some(&"404".to_string()));
}

#[test]
fn test_mcp_server_basic() {
    let mut server = MCPServer::new();

    let tool = MCPTool::new("hello".to_string(), "Says hello".to_string()).add_parameter(
        "name",
        "string",
        "Name to greet",
        true,
    );

    server
        .register_tool(tool, |params, _ctx| {
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("World");

            Ok(ToolResult::success(serde_json::json!({
                "message": format!("Hello, {}!", name)
            })))
        })
        .unwrap();

    assert_eq!(server.list_tools().len(), 1);
    assert!(server.get_tool("hello").is_some());
}

#[test]
fn test_mcp_server_tool_execution() {
    let mut server = MCPServer::new();

    let tool = MCPTool::new("add_numbers".to_string(), "Adds two numbers".to_string())
        .add_parameter("a", "number", "First number", true)
        .add_parameter("b", "number", "Second number", true);

    server
        .register_tool(tool, |params, _ctx| {
            let a = params.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = params.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);

            Ok(ToolResult::success(serde_json::json!({
                "result": a + b
            })))
        })
        .unwrap();

    let ctx = ToolContext::new();
    let params = serde_json::json!({"a": 5, "b": 3});
    let result = server.execute_tool("add_numbers", params, &ctx).unwrap();

    assert!(result.success);
    assert_eq!(result.data.get("result").unwrap().as_f64().unwrap(), 8.0);
}

#[test]
fn test_mcp_server_multiple_tools() {
    let mut server = MCPServer::new();

    // Tool 1
    let tool1 = MCPTool::new("tool1".to_string(), "First tool".to_string());
    server
        .register_tool(tool1, |_params, _ctx| {
            Ok(ToolResult::success(serde_json::json!({"tool": 1})))
        })
        .unwrap();

    // Tool 2
    let tool2 = MCPTool::new("tool2".to_string(), "Second tool".to_string());
    server
        .register_tool(tool2, |_params, _ctx| {
            Ok(ToolResult::success(serde_json::json!({"tool": 2})))
        })
        .unwrap();

    // Tool 3
    let tool3 = MCPTool::new("tool3".to_string(), "Third tool".to_string());
    server
        .register_tool(tool3, |_params, _ctx| {
            Ok(ToolResult::success(serde_json::json!({"tool": 3})))
        })
        .unwrap();

    assert_eq!(server.list_tools().len(), 3);

    let ctx = ToolContext::new();
    let result1 = server
        .execute_tool("tool1", serde_json::json!({}), &ctx)
        .unwrap();
    let result2 = server
        .execute_tool("tool2", serde_json::json!({}), &ctx)
        .unwrap();
    let result3 = server
        .execute_tool("tool3", serde_json::json!({}), &ctx)
        .unwrap();

    assert_eq!(result1.data.get("tool").unwrap().as_u64().unwrap(), 1);
    assert_eq!(result2.data.get("tool").unwrap().as_u64().unwrap(), 2);
    assert_eq!(result3.data.get("tool").unwrap().as_u64().unwrap(), 3);
}

#[test]
fn test_mcp_builtin_tools_registered() {
    let mut server = MCPServer::new();
    server.register_builtin_tools().unwrap();

    let tools = server.list_tools();
    assert!(tools.len() >= 4);

    // Verify specific tools exist
    assert!(server.get_tool("search_documents").is_some());
    assert!(server.get_tool("add_document").is_some());
    assert!(server.get_tool("chunk_text").is_some());
    assert!(server.get_tool("execute_rule").is_some());
}

#[test]
fn test_builtin_search_documents_tool() {
    let mut server = MCPServer::new();
    server.register_builtin_tools().unwrap();

    let ctx = ToolContext::new();
    let params = serde_json::json!({
        "query": "machine learning",
        "top_k": 10,
        "threshold": 0.7
    });

    let result = server
        .execute_tool("search_documents", params, &ctx)
        .unwrap();
    assert!(result.success);
    assert_eq!(
        result.data.get("query").unwrap().as_str().unwrap(),
        "machine learning"
    );
    assert_eq!(result.data.get("top_k").unwrap().as_u64().unwrap(), 10);
}

#[test]
fn test_builtin_add_document_tool() {
    let mut server = MCPServer::new();
    server.register_builtin_tools().unwrap();

    let ctx = ToolContext::new();
    let params = serde_json::json!({
        "id": "doc123",
        "content": "This is a test document",
        "metadata": {"type": "article"}
    });

    let result = server.execute_tool("add_document", params, &ctx).unwrap();
    assert!(result.success);
    assert_eq!(result.data.get("id").unwrap().as_str().unwrap(), "doc123");
    assert_eq!(
        result.data.get("status").unwrap().as_str().unwrap(),
        "accepted"
    );
}

#[test]
fn test_builtin_chunk_text_tool() {
    let mut server = MCPServer::new();
    server.register_builtin_tools().unwrap();

    let ctx = ToolContext::new();
    let params = serde_json::json!({
        "text": "This is a long text that needs to be chunked into smaller pieces.",
        "chunk_size": 20,
        "overlap": 5
    });

    let result = server.execute_tool("chunk_text", params, &ctx).unwrap();
    assert!(result.success);
    assert_eq!(result.data.get("chunk_size").unwrap().as_u64().unwrap(), 20);
    assert_eq!(result.data.get("overlap").unwrap().as_u64().unwrap(), 5);
    assert!(result.data.get("num_chunks").unwrap().as_u64().unwrap() > 0);
}

#[test]
fn test_builtin_chunk_text_default_params() {
    let mut server = MCPServer::new();
    server.register_builtin_tools().unwrap();

    let ctx = ToolContext::new();
    let params = serde_json::json!({
        "text": "Short text"
    });

    let result = server.execute_tool("chunk_text", params, &ctx).unwrap();
    assert!(result.success);
    assert_eq!(
        result.data.get("chunk_size").unwrap().as_u64().unwrap(),
        512
    ); // Default
    assert_eq!(result.data.get("overlap").unwrap().as_u64().unwrap(), 50); // Default
}

#[test]
fn test_builtin_execute_rule_tool() {
    let mut server = MCPServer::new();
    server.register_builtin_tools().unwrap();

    let ctx = ToolContext::new();
    let params = serde_json::json!({
        "rule_id": "rule123",
        "context": {
            "status": "active",
            "confidence": 0.95
        }
    });

    let result = server.execute_tool("execute_rule", params, &ctx).unwrap();
    assert!(result.success);
    assert_eq!(
        result.data.get("rule_id").unwrap().as_str().unwrap(),
        "rule123"
    );
    assert_eq!(
        result.data.get("status").unwrap().as_str().unwrap(),
        "accepted"
    );
}

#[test]
fn test_mcp_tool_not_found() {
    let server = MCPServer::new();
    let ctx = ToolContext::new();

    let result = server.execute_tool("nonexistent_tool", serde_json::json!({}), &ctx);
    assert!(result.is_err());
}

#[test]
fn test_tool_with_context() {
    let mut server = MCPServer::new();

    let tool = MCPTool::new(
        "context_reader".to_string(),
        "Reads from context".to_string(),
    );

    server
        .register_tool(tool, |_params, ctx| {
            let kb_id = ctx
                .knowledge_base
                .clone()
                .unwrap_or_else(|| "none".to_string());

            Ok(ToolResult::success(serde_json::json!({
                "knowledge_base": kb_id,
                "has_store": ctx.document_store.is_some(),
                "data_count": ctx.data.len()
            })))
        })
        .unwrap();

    let ctx = ToolContext::new()
        .with_knowledge_base("kb_test".to_string())
        .with_data("key1".to_string(), "value1".to_string());

    let result = server
        .execute_tool("context_reader", serde_json::json!({}), &ctx)
        .unwrap();
    assert!(result.success);
    assert_eq!(
        result.data.get("knowledge_base").unwrap().as_str().unwrap(),
        "kb_test"
    );
    assert_eq!(result.data.get("data_count").unwrap().as_u64().unwrap(), 1);
}
