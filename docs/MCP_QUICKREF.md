# MCP & Tools Quick Reference

## Setup (30 seconds)

```rust
use ironvault::rag::*;

// Create server and context
let mut server = MCPServer::new();
server.register_builtin_tools()?;

let ctx = ToolContext::new()
    .with_knowledge_base("my_kb".to_string());
```

## Built-in Tools

### search_documents
```rust
server.execute_tool("search_documents", serde_json::json!({
    "query": "AI models",
    "top_k": 5,
    "threshold": 0.7
}), &ctx)?;
```

### add_document
```rust
server.execute_tool("add_document", serde_json::json!({
    "id": "doc1",
    "content": "Document text",
    "metadata": {"type": "research"}
}), &ctx)?;
```

### chunk_text
```rust
server.execute_tool("chunk_text", serde_json::json!({
    "text": "Long text...",
    "chunk_size": 512,
    "overlap": 50
}), &ctx)?;
```

### execute_rule
```rust
server.execute_tool("execute_rule", serde_json::json!({
    "rule_id": "validation",
    "context": {"score": 0.9}
}), &ctx)?;
```

## Custom Tool (1 minute)

```rust
// 1. Define tool
let tool = MCPTool::new("my_tool".to_string(), "Description".to_string())
    .add_parameter("input", "string", "Input text", true);

// 2. Register with executor
server.register_tool(tool, |params, ctx| {
    let input = params.get("input").and_then(|v| v.as_str()).unwrap();
    
    // Your logic here
    let output = input.to_uppercase();
    
    Ok(ToolResult::success(serde_json::json!({
        "result": output
    })))
})?;

// 3. Execute
server.execute_tool("my_tool", 
    serde_json::json!({"input": "hello"}), &ctx)?;
```

## Tool Context Patterns

### Basic
```rust
let ctx = ToolContext::new()
    .with_knowledge_base("kb_id".to_string());
```

### Full Context
```rust
let ctx = ToolContext::new()
    .with_document_store("store_id".to_string())
    .with_knowledge_base("kb_id".to_string())
    .with_data("user_id".to_string(), "user123".to_string())
    .with_data("session".to_string(), "sess_abc".to_string());
```

## Result Handling

```rust
let result = server.execute_tool("tool", params, &ctx)?;

if result.success {
    println!("Data: {:?}", result.data);
} else {
    eprintln!("Error: {:?}", result.error);
}
```

## Parameter Types

| Type      | Example            |
| --------- | ------------------ |
| `string`  | `"text"`           |
| `number`  | `42`, `3.14`       |
| `boolean` | `true`, `false`    |
| `array`   | `["a", "b"]`       |
| `object`  | `{"key": "value"}` |

## Tool Definition Builder

```rust
MCPTool::new("name".to_string(), "desc".to_string())
    .add_parameter("param1", "string", "Description", true)  // required
    .add_parameter("param2", "number", "Description", false) // optional
    .with_metadata("version".to_string(), "1.0".to_string())
    .with_metadata("category".to_string(), "search".to_string())
```

## Common Patterns

### Validation Pattern
```rust
server.register_tool(tool, |params, ctx| {
    let value = params.get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| VaultError::InvalidInput("Missing value".to_string()))?;
    
    if value.is_empty() {
        return Ok(ToolResult::failure("Value required".to_string()));
    }
    
    Ok(ToolResult::success(serde_json::json!({"ok": true})))
})?;
```

### Context-Aware Pattern
```rust
server.register_tool(tool, |params, ctx| {
    let kb = ctx.knowledge_base.clone().unwrap_or_default();
    let user = ctx.data.get("user_id").cloned().unwrap_or_default();
    
    Ok(ToolResult::success(serde_json::json!({
        "kb": kb,
        "user": user
    })))
})?;
```

### Multi-Step Pattern
```rust
server.register_tool(tool, |params, ctx| {
    let steps = params.get("steps")
        .and_then(|v| v.as_array())
        .ok_or_else(|| VaultError::InvalidInput("Missing steps".to_string()))?;
    
    let mut results = Vec::new();
    for step in steps {
        // Process each step
        results.push(process_step(step)?);
    }
    
    Ok(ToolResult::success(serde_json::json!({
        "results": results,
        "count": results.len()
    })))
})?;
```

## Discovery

```rust
// List all tools
let tools = server.list_tools();
for tool in tools {
    println!("{}: {}", tool.name, tool.description);
}

// Get specific tool
if let Some(tool) = server.get_tool("search_documents") {
    println!("Found: {}", tool.name);
}
```

## RAG Pipeline Example

```rust
// 1. Setup
let mut server = MCPServer::new();
server.register_builtin_tools()?;
let ctx = ToolContext::new().with_knowledge_base("kb".to_string());

// 2. Add document
server.execute_tool("add_document", serde_json::json!({
    "id": "doc1",
    "content": "AI is transforming industries"
}), &ctx)?;

// 3. Search
let result = server.execute_tool("search_documents", serde_json::json!({
    "query": "AI transformation",
    "top_k": 3
}), &ctx)?;

// 4. Process results
if result.success {
    println!("Found: {:?}", result.data);
}
```

## Cheat Sheet

| Task              | Code                                         |
| ----------------- | -------------------------------------------- |
| Create server     | `MCPServer::new()`                           |
| Register built-in | `server.register_builtin_tools()?`           |
| Create context    | `ToolContext::new()`                         |
| Add KB to context | `.with_knowledge_base("id".to_string())`     |
| Execute tool      | `server.execute_tool("name", params, &ctx)?` |
| Success result    | `ToolResult::success(json!({}))`             |
| Failure result    | `ToolResult::failure("error".to_string())`   |
| List tools        | `server.list_tools()`                        |
| Get tool          | `server.get_tool("name")`                    |

## Error Handling

```rust
match server.execute_tool("tool", params, &ctx) {
    Ok(result) if result.success => {
        // Handle success
        println!("{:?}", result.data);
    }
    Ok(result) => {
        // Handle tool failure
        eprintln!("Tool failed: {:?}", result.error);
    }
    Err(e) => {
        // Handle execution error
        eprintln!("Error: {}", e);
    }
}
```

## Performance Tips

1. **Reuse server**: Create once, use many times
2. **Minimal context**: Only add needed data
3. **Validate early**: Check params before processing
4. **Clear errors**: Return descriptive failure messages
5. **Use metadata**: Add execution time, version info

## Next Steps

- See [MCP_TOOLS.md](MCP_TOOLS.md) for complete documentation
- See [examples/mcp_tools_demo.rs](https://github.com/nervosys/IronVault/blob/master/examples/mcp_tools_demo.rs) for working examples
- See [tests/rag_tests.rs](https://github.com/nervosys/IronVault/blob/master/tests/rag_tests.rs) for test patterns
