import CodeBlock from "@/components/DocElements";
import { Callout } from "@/components/DocElements";

export default function DatabasePage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Database Backends</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        IronVault ships with two embedded database backends for the RAG
        document store and knowledge base. Both run in-process with zero
        external services.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="backend-comparison">Backend Comparison</h2>
      <div className="overflow-x-auto mb-6">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left py-2 pr-4 font-semibold">Feature</th>
              <th className="text-left py-2 pr-4 font-semibold">SQLite (default)</th>
              <th className="text-left py-2 font-semibold">Sled</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4">Cargo feature</td><td className="py-2 pr-4 font-mono">sqlite</td><td className="py-2 font-mono">kv-store</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4">Storage format</td><td className="py-2 pr-4">Single .db file</td><td className="py-2">Directory of log-structured trees</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4">Best for</td><td className="py-2 pr-4">Structured queries, SQL access, portability</td><td className="py-2">Pure-Rust, high write throughput</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4">Concurrent readers</td><td className="py-2 pr-4">Unlimited (WAL mode)</td><td className="py-2">Unlimited</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4">Concurrent writers</td><td className="py-2 pr-4">Serialised</td><td className="py-2">Lock-free (CAS)</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4">External deps</td><td className="py-2 pr-4">libsqlite3 (bundled)</td><td className="py-2">None</td></tr>
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cli-commands">CLI Commands</h2>
      <CodeBlock language="bash">{`# Initialise a database
iv database init --path ./db --db-type sqlite   # or sled

# Store documents (PDF, text, markdown)
iv database store --path ./db --input paper.pdf
iv database store --path ./db --input notes.md

# Search with semantic similarity
iv database search --path ./db "attention mechanism"

# List all documents
iv database list --path ./db

# Get a specific document
iv database get --path ./db --id <DOC_ID>

# Delete a document
iv database delete --path ./db --id <DOC_ID>

# Export database contents
iv database export --path ./db --output backup.json

# Import from backup
iv database import --path ./db --input backup.json

# Show database statistics
iv database stats --path ./db`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="sqlite-api">SQLite Rust API</h2>
      <CodeBlock language="rust">{`use ironvault::rag::sqlite_store::SqliteDocumentStore;
use ironvault::rag::document_store::DocumentStore;

// Open or create a SQLite database
let store = SqliteDocumentStore::new("./knowledge.db")?;

// Store a document with metadata
let doc_id = store.store_document(
    "Transformer Architecture",
    "The transformer model relies entirely on self-attention...",
    serde_json::json!({ "source": "paper", "year": 2017 }),
)?;

// Full-text search
let results = store.search("self-attention mechanism", 5)?;

// Retrieve by ID
let doc = store.get_document(&doc_id)?;`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="sled-api">Sled Rust API</h2>
      <CodeBlock language="rust">{`use ironvault::rag::kv_store::SledDocumentStore;
use ironvault::rag::document_store::DocumentStore;

// Open or create a Sled database
let store = SledDocumentStore::new("./knowledge_sled")?;

// Same DocumentStore trait — identical interface
let doc_id = store.store_document(
    "RLHF Training",
    "Reinforcement Learning from Human Feedback...",
    serde_json::json!({ "topic": "alignment" }),
)?;

let results = store.search("reward model", 5)?;`}</CodeBlock>
      <Callout type="tip" title="Swappable backends">
        Both backends implement the same <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">DocumentStore</code> trait,
        so you can switch between SQLite and Sled by changing one line.
      </Callout>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="embeddings">Embeddings &amp; Similarity</h2>
      <p className="mb-4 text-[var(--color-text-secondary)]">
        Documents are automatically chunked and embedded when stored. The
        search command uses cosine similarity over these embeddings to return
        the most relevant chunks.
      </p>
      <CodeBlock language="rust">{`use ironvault::rag::embeddings;

// Generate embeddings for a text chunk
let vector = embeddings::generate("transformer attention")?;

// Compute cosine similarity
let score = embeddings::cosine_similarity(&vec_a, &vec_b);`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="document-schema">Document Schema</h2>
      <CodeBlock language="rust">{`pub struct Document {
    pub id: String,           // UUID
    pub title: String,        // Human-readable title
    pub content: String,      // Full document text
    pub metadata: Value,      // Arbitrary JSON metadata
    pub created_at: DateTime, // Timestamp
    pub updated_at: DateTime, // Last modification
}

pub struct ChunkInfo {
    pub chunk_id: String,     // UUID
    pub doc_id: String,       // Parent document
    pub content: String,      // Chunk text
    pub embedding: Vec<f32>,  // Vector embedding
    pub position: usize,      // Chunk index in document
}`}</CodeBlock>
    </>
  );
}
