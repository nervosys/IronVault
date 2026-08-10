# MCP (Model Context Protocol) & Tools Guide

## Overview

IronVault provides comprehensive support for **Model Context Protocol (MCP)** and tool-based interactions, enabling AI agents and applications to execute structured operations on RAG systems, knowledge bases, and custom workflows.

## Table of Contents

- [Quick Start](#quick-start)
- [MCP Tools](#mcp-tools)
- [Tool Definition](#tool-definition)
- [Tool Execution](#tool-execution)
- [Built-in Tools](#built-in-tools)
- [Custom Tools](#custom-tools)
- [Tool Context](#tool-context)
- [MCP Server](#mcp-server)
- [Complete Examples](#complete-examples)

## Quick Start

```rust
use ironvault::rag::*;

// Create MCP server
let mut server = MCPServer::new();

// Register built-in RAG tools
server.register_builtin_tools()?;

// Create tool context
let ctx = ToolContext::new()
    .with_knowledge_base("my_kb".to_string());

// Execute a tool
let result = server.execute_tool(
    "search_documents",
    serde_json::json!({"query": "machine learning", "top_k": 5}),
    &ctx
)?;

println!("Results: {:?}", result.data);
```

## MCP Tools

### What is MCP?

Model Context Protocol (MCP) provides a standardized way for AI models and agents to interact with external tools and services. In IronVault, MCP enables:

- **Structured Tool Invocation**: Define tools with clear input/output schemas
- **Context Management**: Pass contextual information to tools
- **Result Standardization**: Consistent result format across all tools
- **Error Handling**: Proper error reporting and handling

### Tool Architecture

```
┌─────────────┐
│  AI Agent   │
└──────┬──────┘
       │
       ▼
┌─────────────┐      ┌──────────────┐
│ MCP Server  │◄────►│  Tool Store  │
└──────┬──────┘      └──────────────┘
       │
       ▼
┌─────────────┐      ┌──────────────┐
│  Executor   │◄────►│   Context    │
└──────┬──────┘      └──────────────┘
       │
       ▼
┌─────────────┐
│   Result    │
└─────────────┘
```

## Tool Definition

### Creating a Tool

```rust
let tool = MCPTool::new(
    "search_documents".to_string(),
    "Search for documents using semantic similarity".to_string(),
)
.add_parameter("query", "string", "The search query", true)
.add_parameter("top_k", "number", "Number of results to return", false)
.add_parameter("threshold", "number", "Minimum similarity score", false)
.with_metadata("version".to_string(), "1.0".to_string())
.with_metadata("category".to_string(), "retrieval".to_string());
```

### Parameter Types

MCP tools support standard JSON Schema types:

- `string` - Text values
- `number` - Numeric values (int or float)
- `boolean` - True/false values
- `array` - Lists of values
- `object` - Nested objects/maps

### Required vs Optional Parameters

```rust
// Required parameter (third argument: true)
.add_parameter("query", "string", "Search query", true)

// Optional parameter (third argument: false)
.add_parameter("top_k", "number", "Max results", false)
```

### Tool Metadata

Add custom metadata to tools for versioning, categorization, or other purposes:

```rust
tool.with_metadata("version".to_string(), "1.0".to_string())
    .with_metadata("author".to_string(), "Team AI".to_string())
    .with_metadata("category".to_string(), "search".to_string())
```

## Tool Execution

### Tool Context

The `ToolContext` provides contextual information to tool executors:

```rust
let ctx = ToolContext::new()
    .with_document_store("store_id".to_string())
    .with_knowledge_base("kb_id".to_string())
    .with_data("user_id".to_string(), "user123".to_string())
    .with_data("session".to_string(), "abc123".to_string());
```

### Tool Results

Tools return `ToolResult` with standardized structure:

```rust
// Success
let success = ToolResult::success(serde_json::json!({
    "documents": ["doc1", "doc2"],
    "count": 2
}))
.with_metadata("execution_time".to_string(), "45ms".to_string());

// Failure
let failure = ToolResult::failure("Document not found".to_string())
    .with_metadata("error_code".to_string(), "404".to_string());
```

### Executing Tools

```rust
let result = server.execute_tool(
    "tool_name",
    serde_json::json!({"param1": "value1"}),
    &ctx
)?;

if result.success {
    println!("Success: {:?}", result.data);
} else {
    eprintln!("Error: {:?}", result.error);
}
```

## Built-in Tools

IronVault includes 4 built-in RAG tools:

### 1. search_documents

Search for similar documents using vector embeddings.

**Parameters:**
- `query` (string, required): Search query text
- `top_k` (number, optional): Number of results (default: 5)
- `threshold` (number, optional): Minimum similarity threshold

**Example:**
```rust
let result = server.execute_tool(
    "search_documents",
    serde_json::json!({
        "query": "machine learning algorithms",
        "top_k": 10,
        "threshold": 0.7
    }),
    &ctx
)?;
```

### 2. add_document

Add a document to the knowledge base.

**Parameters:**
- `id` (string, required): Document ID
- `content` (string, required): Document content
- `metadata` (object, optional): Document metadata

**Example:**
```rust
let result = server.execute_tool(
    "add_document",
    serde_json::json!({
        "id": "doc123",
        "content": "Artificial intelligence is...",
        "metadata": {"category": "AI"}
    }),
    &ctx
)?;
```

### 3. chunk_text

Split text into chunks for processing.

**Parameters:**
- `text` (string, required): Text to chunk
- `chunk_size` (number, optional): Size of each chunk (default: 512)
- `overlap` (number, optional): Overlap between chunks (default: 50)

**Example:**
```rust
let result = server.execute_tool(
    "chunk_text",
    serde_json::json!({
        "text": "Long document text...",
        "chunk_size": 256,
        "overlap": 25
    }),
    &ctx
)?;
```

### 4. execute_rule

Execute a business rule with given context.

**Parameters:**
- `rule_id` (string, required): Rule identifier
- `context` (object, required): Rule execution context

**Example:**
```rust
let result = server.execute_tool(
    "execute_rule",
    serde_json::json!({
        "rule_id": "confidence_check",
        "context": {
            "confidence": 0.95,
            "model": "gpt-4"
        }
    }),
    &ctx
)?;
```

## Custom Tools

### Creating Custom Tool Executors

```rust
let mut server = MCPServer::new();

// Define the tool
let custom_tool = MCPTool::new(
    "analyze_sentiment".to_string(),
    "Analyze text sentiment".to_string(),
)
.add_parameter("text", "string", "Text to analyze", true);

// Register with executor function
server.register_tool(custom_tool, |params, ctx| {
    let text = params.get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| VaultError::InvalidInput("Missing text".to_string()))?;

    // Custom logic here
    let sentiment = if text.contains("good") { "positive" } else { "neutral" };

    Ok(ToolResult::success(serde_json::json!({
        "sentiment": sentiment,
        "confidence": 0.85,
        "text_length": text.len()
    })))
})?;
```

### Complex Custom Tool Example

```rust
// Multi-step processing tool
let processor_tool = MCPTool::new(
    "process_document".to_string(),
    "Complete document processing pipeline".to_string(),
)
.add_parameter("doc_id", "string", "Document ID", true)
.add_parameter("steps", "array", "Processing steps", true);

server.register_tool(processor_tool, |params, ctx| {
    let doc_id = params.get("doc_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| VaultError::InvalidInput("Missing doc_id".to_string()))?;

    let steps = params.get("steps")
        .and_then(|v| v.as_array())
        .ok_or_else(|| VaultError::InvalidInput("Missing steps".to_string()))?;

    let mut results = Vec::new();
    for step in steps {
        let step_name = step.as_str().unwrap_or("unknown");
        // Process each step
        results.push(format!("Completed: {}", step_name));
    }

    Ok(ToolResult::success(serde_json::json!({
        "doc_id": doc_id,
        "steps_completed": results.len(),
        "results": results
    }))
    .with_metadata("kb".to_string(), ctx.knowledge_base.clone().unwrap_or_default()))
})?;
```

## Tool Context

### Context-Aware Tools

Tools can access context information to customize behavior:

```rust
let context_tool = MCPTool::new(
    "get_user_docs".to_string(),
    "Get documents for current user".to_string(),
);

server.register_tool(context_tool, |_params, ctx| {
    let user_id = ctx.data.get("user_id")
        .cloned()
        .unwrap_or_else(|| "anonymous".to_string());

    let kb_id = ctx.knowledge_base
        .clone()
        .unwrap_or_else(|| "default".to_string());

    Ok(ToolResult::success(serde_json::json!({
        "user_id": user_id,
        "knowledge_base": kb_id,
        "documents": [] // Would fetch from actual KB
    })))
})?;
```

### Building Rich Context

```rust
let ctx = ToolContext::new()
    // System context
    .with_document_store("production_store".to_string())
    .with_knowledge_base("main_kb".to_string())
    
    // User context
    .with_data("user_id".to_string(), "user_123".to_string())
    .with_data("role".to_string(), "admin".to_string())
    
    // Session context
    .with_data("session_id".to_string(), "sess_abc".to_string())
    .with_data("timestamp".to_string(), "2025-10-28T12:00:00Z".to_string())
    
    // Custom context
    .with_data("language".to_string(), "en".to_string())
    .with_data("region".to_string(), "us-east".to_string());
```

## MCP Server

### Server Setup

```rust
let mut server = MCPServer::new();

// Register built-in tools
server.register_builtin_tools()?;

// Register custom tools
server.register_tool(my_tool, my_executor)?;

// List all tools
let tools = server.list_tools();
for tool in tools {
    println!("Tool: {} - {}", tool.name, tool.description);
}

// Get specific tool
if let Some(tool) = server.get_tool("search_documents") {
    println!("Found: {}", tool.name);
}
```

### Tool Discovery

```rust
// List all available tools
let tools = server.list_tools();
println!("Available tools: {}", tools.len());

for tool in tools {
    println!("\n{}", tool.name);
    println!("  Description: {}", tool.description);
    
    if let Some(props) = tool.input_schema.get("properties") {
        if let Some(obj) = props.as_object() {
            println!("  Parameters:");
            for (name, schema) in obj {
                println!("    - {}: {:?}", name, schema.get("type"));
            }
        }
    }
}
```

### Error Handling

```rust
match server.execute_tool("my_tool", params, &ctx) {
    Ok(result) => {
        if result.success {
            println!("Success: {:?}", result.data);
        } else {
            eprintln!("Tool failed: {:?}", result.error);
        }
    }
    Err(e) => {
        eprintln!("Execution error: {}", e);
    }
}
```

## Complete Examples

### RAG Query Pipeline with MCP

```rust
use ironvault::rag::*;

fn rag_pipeline_with_mcp(query: &str) -> Result<String, Box<dyn std::error::Error>> {
    // 1. Setup MCP server
    let mut server = MCPServer::new();
    server.register_builtin_tools()?;
    
    // 2. Create context
    let ctx = ToolContext::new()
        .with_knowledge_base("research_kb".to_string())
        .with_data("user".to_string(), "researcher_1".to_string());
    
    // 3. Chunk the query if needed
    let chunk_result = server.execute_tool(
        "chunk_text",
        serde_json::json!({
            "text": query,
            "chunk_size": 100
        }),
        &ctx
    )?;
    
    // 4. Search for relevant documents
    let search_result = server.execute_tool(
        "search_documents",
        serde_json::json!({
            "query": query,
            "top_k": 5,
            "threshold": 0.6
        }),
        &ctx
    )?;
    
    // 5. Generate response (simulated)
    Ok(format!("Generated response for: {}", query))
}
```

### Custom Agent with Tools

```rust
struct AIAgent {
    server: MCPServer,
    context: ToolContext,
}

impl AIAgent {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut server = MCPServer::new();
        server.register_builtin_tools()?;
        
        // Add agent-specific tools
        let think_tool = MCPTool::new(
            "think".to_string(),
            "Internal reasoning step".to_string(),
        )
        .add_parameter("thought", "string", "Reasoning", true);
        
        server.register_tool(think_tool, |params, _ctx| {
            let thought = params.get("thought")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            Ok(ToolResult::success(serde_json::json!({
                "thought": thought,
                "processed": true
            })))
        })?;
        
        let context = ToolContext::new()
            .with_knowledge_base("agent_kb".to_string());
        
        Ok(Self { server, context })
    }
    
    fn execute(&self, tool: &str, params: serde_json::Value) -> Result<ToolResult, Box<dyn std::error::Error>> {
        Ok(self.server.execute_tool(tool, params, &self.context)?)
    }
    
    fn available_tools(&self) -> Vec<String> {
        self.server.list_tools()
            .iter()
            .map(|t| t.name.clone())
            .collect()
    }
}
```

## Best Practices

### 1. Tool Design

- **Single Responsibility**: Each tool should do one thing well
- **Clear Parameters**: Use descriptive parameter names and descriptions
- **Proper Validation**: Validate all input parameters
- **Error Messages**: Provide clear, actionable error messages

### 2. Context Management

- **Minimal Context**: Only include necessary context data
- **Consistent Keys**: Use standardized key names across tools
- **Immutable Context**: Treat context as read-only in executors

### 3. Error Handling

```rust
server.register_tool(tool, |params, ctx| {
    // Validate parameters
    let value = params.get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| VaultError::InvalidInput("Missing value".to_string()))?;
    
    // Validate business logic
    if value.is_empty() {
        return Ok(ToolResult::failure("Value cannot be empty".to_string()));
    }
    
    // Execute with error handling
    match process_value(value) {
        Ok(result) => Ok(ToolResult::success(result)),
        Err(e) => Ok(ToolResult::failure(e.to_string())),
    }
})?;
```

### 4. Performance

- **Lazy Evaluation**: Only compute what's needed
- **Caching**: Cache expensive operations
- **Async Operations**: Use async for I/O-bound tools (future enhancement)
- **Resource Limits**: Set timeouts and size limits

## Integration Patterns

### With Knowledge Bases

```rust
let kb = KnowledgeBase::new("kb".to_string(), config);

// Create tool that uses KB
let kb_tool = MCPTool::new("kb_search".to_string(), "Search KB".to_string())
    .add_parameter("query", "string", "Query", true);

server.register_tool(kb_tool, move |params, _ctx| {
    let query = params.get("query").and_then(|v| v.as_str()).unwrap();
    // Use kb.retrieve() here
    Ok(ToolResult::success(serde_json::json!({"results": []})))
})?;
```

### With Rule Engines

```rust
let mut engine = RuleEngine::new();

// Tool to execute rules
let rule_tool = MCPTool::new("apply_rules".to_string(), "Apply rules".to_string())
    .add_parameter("context", "object", "Rule context", true);

server.register_tool(rule_tool, move |params, _ctx| {
    // Execute rules with engine
    Ok(ToolResult::success(serde_json::json!({"rules_applied": 0})))
})?;
```

## API Reference

### MCPTool

```rust
impl MCPTool {
    pub fn new(name: String, description: String) -> Self;
    pub fn add_parameter(self, name: &str, type: &str, desc: &str, required: bool) -> Self;
    pub fn with_metadata(self, key: String, value: String) -> Self;
}
```

### ToolContext

```rust
impl ToolContext {
    pub fn new() -> Self;
    pub fn with_document_store(self, store_id: String) -> Self;
    pub fn with_knowledge_base(self, kb_id: String) -> Self;
    pub fn with_data(self, key: String, value: String) -> Self;
}
```

### ToolResult

```rust
impl ToolResult {
    pub fn success(data: JsonValue) -> Self;
    pub fn failure(error: String) -> Self;
    pub fn with_metadata(self, key: String, value: String) -> Self;
}
```

### MCPServer

```rust
impl MCPServer {
    pub fn new() -> Self;
    pub fn register_tool<F>(&mut self, tool: MCPTool, executor: F) -> Result<()>;
    pub fn execute_tool(&self, name: &str, params: JsonValue, ctx: &ToolContext) -> Result<ToolResult>;
    pub fn list_tools(&self) -> Vec<&MCPTool>;
    pub fn get_tool(&self, name: &str) -> Option<&MCPTool>;
    pub fn register_builtin_tools(&mut self) -> Result<()>;
}
```

## See Also

- [RAG Guide](RAG.md) - Complete RAG documentation
- [RAG Quick Reference](RAG_QUICKREF.md) - Quick reference card
- [Examples](https://github.com/nervosys/IronVault/blob/master/examples/mcp_tools_demo.rs) - Working examples
- [Tests](https://github.com/nervosys/IronVault/blob/master/tests/rag_tests.rs) - Test suite
