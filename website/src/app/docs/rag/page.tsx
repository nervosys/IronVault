import CodeBlock from "@/components/DocElements";

export default function RAGPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">RAG & MCP Tools</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Retrieval-Augmented Generation with document stores, knowledge bases, and Model Context Protocol agent tools.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="document-store">Document Store</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        In-memory document store with vector embeddings for semantic search.
      </p>
      <CodeBlock language="rust">{`use ironvault::rag::{DocumentStore, Document};

let mut store = DocumentStore::new();

// Add documents
store.add(Document::new("doc-1", "Neural networks are computational models..."));
store.add(Document::new("doc-2", "Transformers use self-attention mechanisms..."));

// Semantic search
let results = store.search("attention mechanism", 5)?;
for result in results {
    println!("{}: score={:.3}", result.id, result.score);
}`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="knowledge-base">Knowledge Base</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Automatic text chunking with configurable parameters.
      </p>
      <CodeBlock language="rust">{`use ironvault::rag::KnowledgeBase;

let mut kb = KnowledgeBase::new();

// Add a large document — auto-chunked
kb.add_document("paper.txt", &large_text)?;

// Retrieve relevant chunks
let chunks = kb.retrieve("transformer architecture", 3)?;`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="mcp">MCP Tools</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Model Context Protocol server for AI agent integrations. Register custom tools for agent execution.
      </p>
      <CodeBlock language="rust">{`use ironvault::rag::{McpServer, McpTool, ToolResult};

let mut server = McpServer::new("vault-agent");

// Built-in tools are registered automatically
// - search_documents: Semantic search across documents
// - chunk_text: Split text into chunks
// - add_document: Add a document to the store
// - execute_rule: Run a rule engine action

// Register a custom tool
server.register(McpTool::new(
    "analyze-model",
    "Analyze a stored model",
    |params| {
        // Tool logic
        Ok(ToolResult::success("Analysis complete"))
    },
));

// Execute a tool
let result = server.execute("search_documents", &params)?;`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="built-in-tools">Built-in Tools</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Tool</th>
              <th className="text-left p-3 font-semibold">Description</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["search_documents", "Semantic search across document store"],
              ["chunk_text", "Split text into chunks with configurable size and overlap"],
              ["add_document", "Add a new document to the knowledge base"],
              ["execute_rule", "Run a rule engine action on input data"],
            ].map(([tool, desc]) => (
              <tr key={tool} className="border-b border-[var(--color-border)]">
                <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{tool}</code></td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="rule-engine">Rule Engine</h2>
      <CodeBlock language="rust">{`use ironvault::rag::RuleEngine;

let mut engine = RuleEngine::new();

// Add rules with conditions
engine.add_rule("large-model", |ctx| {
    ctx.get("parameters")
        .and_then(|v| v.parse::<u64>().ok())
        .map(|p| p > 1_000_000_000)
        .unwrap_or(false)
}, "Consider quantization for models with >1B parameters");

// Evaluate
let actions = engine.evaluate(&context)?;`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cache">Retrieval Cache</h2>
      <p className="text-[var(--color-text-secondary)]">
        Built-in LRU cache for retrieval results to avoid redundant searches. Cache size is configurable
        and entries are automatically evicted when capacity is reached.
      </p>
    </>
  );
}
