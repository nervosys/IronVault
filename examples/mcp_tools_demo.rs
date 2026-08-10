//! Demonstration of MCP (Model Context Protocol) and Tools
//!
//! This example shows how to:
//! - Create and register MCP tools
//! - Execute tools with parameters
//! - Use built-in RAG tools
//! - Build custom tool executors
//! - Integrate tools with knowledge bases

use ironvault::rag::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IronVault MCP Tools Demo ===\n");

    // PART 1: Basic Tool Creation
    println!("📦 PART 1: Creating Custom Tools");
    println!("--------------------------------");
    demo_tool_creation()?;

    // PART 2: MCP Server Setup
    println!("\n🔧 PART 2: MCP Server Setup");
    println!("---------------------------");
    demo_mcp_server()?;

    // PART 3: Built-in RAG Tools
    println!("\n🤖 PART 3: Built-in RAG Tools");
    println!("----------------------------");
    demo_builtin_tools()?;

    // PART 4: Custom Tool Executors
    println!("\n⚡ PART 4: Custom Tool Executors");
    println!("-------------------------------");
    demo_custom_executors()?;

    // PART 5: Tools with Context
    println!("\n🎯 PART 5: Tools with Context");
    println!("----------------------------");
    demo_tools_with_context()?;

    // PART 6: Complete RAG + MCP Pipeline
    println!("\n🚀 PART 6: Complete RAG + MCP Pipeline");
    println!("-------------------------------------");
    demo_complete_pipeline()?;

    println!("\n✅ All MCP demonstrations completed successfully!");
    Ok(())
}

fn demo_tool_creation() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating MCP tools...");

    // Tool 1: Simple search tool
    let search_tool = MCPTool::new(
        "semantic_search".to_string(),
        "Search documents using semantic similarity".to_string(),
    )
    .add_parameter("query", "string", "The search query", true)
    .add_parameter("top_k", "number", "Number of results", false)
    .add_parameter("min_score", "number", "Minimum similarity score", false)
    .with_metadata("version".to_string(), "1.0".to_string())
    .with_metadata("category".to_string(), "retrieval".to_string());

    println!("✓ Created tool: {}", search_tool.name);
    println!("  Description: {}", search_tool.description);
    println!(
        "  Parameters: {}",
        search_tool
            .input_schema
            .get("properties")
            .unwrap()
            .as_object()
            .unwrap()
            .len()
    );

    // Tool 2: Document analyzer
    let analyze_tool = MCPTool::new(
        "analyze_document".to_string(),
        "Analyze document properties and metadata".to_string(),
    )
    .add_parameter("doc_id", "string", "Document ID to analyze", true)
    .add_parameter(
        "include_embeddings",
        "boolean",
        "Include embedding analysis",
        false,
    );

    println!("✓ Created tool: {}", analyze_tool.name);

    // Tool 3: Batch processor
    let batch_tool = MCPTool::new(
        "batch_process".to_string(),
        "Process multiple documents in batch".to_string(),
    )
    .add_parameter("doc_ids", "array", "List of document IDs", true)
    .add_parameter("operation", "string", "Operation to perform", true);

    println!("✓ Created tool: {}", batch_tool.name);

    println!("\nTotal tools created: 3");
    Ok(())
}

fn demo_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = MCPServer::new();

    println!("Registering tools with MCP server...");

    // Register echo tool
    let echo_tool = MCPTool::new(
        "echo".to_string(),
        "Echo back the input message".to_string(),
    )
    .add_parameter("message", "string", "Message to echo", true);

    server.register_tool(echo_tool, |params, _ctx| {
        let message = params.get("message").and_then(|v| v.as_str()).unwrap_or("");

        Ok(ToolResult::success(serde_json::json!({
            "echo": message,
            "length": message.len()
        })))
    })?;

    println!("✓ Registered: echo");

    // Register calculator tool
    let calc_tool = MCPTool::new(
        "calculate".to_string(),
        "Perform arithmetic calculations".to_string(),
    )
    .add_parameter(
        "operation",
        "string",
        "Operation: add, subtract, multiply, divide",
        true,
    )
    .add_parameter("a", "number", "First operand", true)
    .add_parameter("b", "number", "Second operand", true);

    server.register_tool(calc_tool, |params, _ctx| {
        let op = params
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("add");
        let a = params.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let b = params.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let result = match op {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b != 0.0 {
                    a / b
                } else {
                    f64::NAN
                }
            }
            _ => f64::NAN,
        };

        Ok(ToolResult::success(serde_json::json!({
            "operation": op,
            "operands": [a, b],
            "result": result
        })))
    })?;

    println!("✓ Registered: calculate");

    // Test execution
    println!("\nExecuting tools:");
    let ctx = ToolContext::new();

    // Test echo
    let echo_result =
        server.execute_tool("echo", serde_json::json!({"message": "Hello, MCP!"}), &ctx)?;
    println!("  echo: {:?}", echo_result.data.get("echo"));

    // Test calculator
    let calc_result = server.execute_tool(
        "calculate",
        serde_json::json!({"operation": "multiply", "a": 6, "b": 7}),
        &ctx,
    )?;
    println!("  calculate: 6 × 7 = {:?}", calc_result.data.get("result"));

    println!("\nTotal registered tools: {}", server.list_tools().len());
    Ok(())
}

fn demo_builtin_tools() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = MCPServer::new();
    server.register_builtin_tools()?;

    println!("Built-in tools registered: {}", server.list_tools().len());

    let ctx = ToolContext::new();

    // Test 1: Search documents
    println!("\n1. Testing search_documents:");
    let search_result = server.execute_tool(
        "search_documents",
        serde_json::json!({
            "query": "machine learning algorithms",
            "top_k": 5,
            "threshold": 0.7
        }),
        &ctx,
    )?;
    println!("   Query: {}", search_result.data.get("query").unwrap());
    println!("   Top K: {}", search_result.data.get("top_k").unwrap());
    println!(
        "   Status: {}",
        if search_result.success { "✓" } else { "✗" }
    );

    // Test 2: Add document
    println!("\n2. Testing add_document:");
    let add_result = server.execute_tool(
        "add_document",
        serde_json::json!({
            "id": "doc_ai_001",
            "content": "Introduction to artificial intelligence and neural networks.",
            "metadata": {
                "category": "AI",
                "author": "Research Team"
            }
        }),
        &ctx,
    )?;
    println!("   Document ID: {}", add_result.data.get("id").unwrap());
    println!("   Status: {}", add_result.data.get("status").unwrap());

    // Test 3: Chunk text
    println!("\n3. Testing chunk_text:");
    let text = "This is a demonstration of text chunking. The text will be split into smaller, manageable pieces with configurable overlap to maintain context across chunks.";
    let chunk_result = server.execute_tool(
        "chunk_text",
        serde_json::json!({
            "text": text,
            "chunk_size": 50,
            "overlap": 10
        }),
        &ctx,
    )?;
    println!(
        "   Text length: {}",
        chunk_result.data.get("text_length").unwrap()
    );
    println!(
        "   Chunk size: {}",
        chunk_result.data.get("chunk_size").unwrap()
    );
    println!(
        "   Number of chunks: {}",
        chunk_result.data.get("num_chunks").unwrap()
    );

    // Test 4: Execute rule
    println!("\n4. Testing execute_rule:");
    let rule_result = server.execute_tool(
        "execute_rule",
        serde_json::json!({
            "rule_id": "confidence_threshold",
            "context": {
                "confidence": 0.95,
                "model": "gpt-4",
                "task": "classification"
            }
        }),
        &ctx,
    )?;
    println!("   Rule ID: {}", rule_result.data.get("rule_id").unwrap());
    println!("   Status:  {}", rule_result.data.get("status").unwrap());
    if let Some(note) = rule_result.data.get("note") {
        println!("   Note:    {}", note);
    }

    Ok(())
}

fn demo_custom_executors() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = MCPServer::new();

    println!("Creating custom tool executors...");

    // Custom tool 1: Text statistics
    let stats_tool = MCPTool::new(
        "text_stats".to_string(),
        "Calculate text statistics".to_string(),
    )
    .add_parameter("text", "string", "Text to analyze", true);

    server.register_tool(stats_tool, |params, _ctx| {
        let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");

        let char_count = text.len();
        let word_count = text.split_whitespace().count();
        let line_count = text.lines().count();
        let avg_word_length = if word_count > 0 {
            char_count as f64 / word_count as f64
        } else {
            0.0
        };

        Ok(ToolResult::success(serde_json::json!({
            "characters": char_count,
            "words": word_count,
            "lines": line_count,
            "avg_word_length": avg_word_length
        }))
        .with_metadata("analyzer".to_string(), "custom_text_stats".to_string()))
    })?;

    println!("✓ Registered: text_stats");

    // Custom tool 2: Embedding generator (mock)
    let embed_tool = MCPTool::new(
        "generate_embedding".to_string(),
        "Generate mock embedding vector".to_string(),
    )
    .add_parameter("text", "string", "Text to embed", true)
    .add_parameter("dimensions", "number", "Embedding dimensions", false);

    server.register_tool(embed_tool, |params, _ctx| {
        let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let dims = params
            .get("dimensions")
            .and_then(|v| v.as_u64())
            .unwrap_or(384) as usize;

        // Mock embedding: simple hash-based values
        let mut embedding = vec![0.0; dims];
        for (i, byte) in text.bytes().enumerate() {
            embedding[i % dims] += (byte as f32) / 255.0;
        }

        // Normalize
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embedding {
                *val /= magnitude;
            }
        }

        Ok(ToolResult::success(serde_json::json!({
            "embedding": embedding,
            "dimensions": dims,
            "text_length": text.len()
        })))
    })?;

    println!("✓ Registered: generate_embedding");

    // Test custom tools
    println!("\nTesting custom tools:");
    let ctx = ToolContext::new();

    let stats_result = server.execute_tool(
        "text_stats",
        serde_json::json!({"text": "Hello world! This is a test of the text statistics tool."}),
        &ctx,
    )?;
    println!("  Text stats:");
    println!(
        "    Characters: {}",
        stats_result.data.get("characters").unwrap()
    );
    println!("    Words: {}", stats_result.data.get("words").unwrap());

    let embed_result = server.execute_tool(
        "generate_embedding",
        serde_json::json!({"text": "machine learning", "dimensions": 8}),
        &ctx,
    )?;
    println!("  Embedding generated:");
    println!(
        "    Dimensions: {}",
        embed_result.data.get("dimensions").unwrap()
    );

    Ok(())
}

fn demo_tools_with_context() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = MCPServer::new();

    println!("Creating context-aware tools...");

    // Tool that uses context
    let context_tool = MCPTool::new(
        "get_context_info".to_string(),
        "Retrieve information from tool context".to_string(),
    );

    server.register_tool(context_tool, |_params, ctx| {
        Ok(ToolResult::success(serde_json::json!({
            "has_document_store": ctx.document_store.is_some(),
            "has_knowledge_base": ctx.knowledge_base.is_some(),
            "context_data_keys": ctx.data.keys().collect::<Vec<_>>(),
            "context_data_count": ctx.data.len()
        })))
    })?;

    // Create rich context
    let ctx = ToolContext::new()
        .with_document_store("main_store".to_string())
        .with_knowledge_base("research_kb".to_string())
        .with_data("user_id".to_string(), "user_123".to_string())
        .with_data("session_id".to_string(), "session_abc".to_string())
        .with_data("role".to_string(), "admin".to_string());

    println!("Context created:");
    println!("  Document store: {:?}", ctx.document_store);
    println!("  Knowledge base: {:?}", ctx.knowledge_base);
    println!("  Context data: {} keys", ctx.data.len());

    let result = server.execute_tool("get_context_info", serde_json::json!({}), &ctx)?;
    println!("\nContext info retrieved:");
    println!("  {}", serde_json::to_string_pretty(&result.data)?);

    Ok(())
}

fn demo_complete_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    println!("Building complete RAG + MCP pipeline...");

    // 1. Initialize MCP server with all tools
    let mut server = MCPServer::new();
    server.register_builtin_tools()?;

    // Add custom RAG tool
    let rag_query_tool = MCPTool::new(
        "rag_query".to_string(),
        "Execute a complete RAG query pipeline".to_string(),
    )
    .add_parameter("query", "string", "User query", true)
    .add_parameter("max_context", "number", "Max context documents", false);

    server.register_tool(rag_query_tool, |params, ctx| {
        let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let max_ctx = params
            .get("max_context")
            .and_then(|v| v.as_u64())
            .unwrap_or(3);

        Ok(ToolResult::success(serde_json::json!({
            "query": query,
            "max_context": max_ctx,
            "retrieved_docs": [],
            "generated_response": format!("Response to: {}", query),
            "metadata": {
                "knowledge_base": ctx.knowledge_base.clone().unwrap_or_else(|| "none".to_string()),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        })))
    })?;

    println!(
        "✓ Initialized MCP server with {} tools",
        server.list_tools().len()
    );

    // 2. Create context
    let ctx = ToolContext::new()
        .with_knowledge_base("production_kb".to_string())
        .with_document_store("main_store".to_string())
        .with_data("user".to_string(), "demo_user".to_string());

    // 3. Execute pipeline steps
    println!("\n📋 Executing RAG pipeline:");

    // Step 1: Chunk input text
    println!("\n  Step 1: Chunking input text...");
    let chunk_result = server.execute_tool(
        "chunk_text",
        serde_json::json!({
            "text": "Artificial intelligence encompasses machine learning, deep learning, and neural networks.",
            "chunk_size": 30,
            "overlap": 5
        }),
        &ctx,
    )?;
    println!(
        "    ✓ Created {} chunks",
        chunk_result.data.get("num_chunks").unwrap()
    );

    // Step 2: Search relevant documents
    println!("\n  Step 2: Searching for relevant documents...");
    let search_result = server.execute_tool(
        "search_documents",
        serde_json::json!({
            "query": "What is deep learning?",
            "top_k": 3
        }),
        &ctx,
    )?;
    println!(
        "    ✓ Search completed for: {}",
        search_result.data.get("query").unwrap()
    );

    // Step 3: Execute complete RAG query
    println!("\n  Step 3: Executing RAG query...");
    let rag_result = server.execute_tool(
        "rag_query",
        serde_json::json!({
            "query": "Explain the relationship between AI and machine learning",
            "max_context": 5
        }),
        &ctx,
    )?;
    println!(
        "    ✓ Generated response: {}",
        rag_result
            .data
            .get("generated_response")
            .unwrap()
            .as_str()
            .unwrap()
    );

    // 4. List all available tools
    println!("\n📚 Available tools in this pipeline:");
    for (i, tool) in server.list_tools().iter().enumerate() {
        println!("  {}. {} - {}", i + 1, tool.name, tool.description);
    }

    println!("\n✨ Pipeline completed successfully!");

    Ok(())
}
