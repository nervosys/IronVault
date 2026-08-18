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
      <CodeBlock language="rust">{`// requires --features sqlite
use ironvault::rag::{Database, SQLiteDatabase};
use std::collections::HashMap;
use std::path::Path;

let mut db = SQLiteDatabase::new(Path::new("./knowledge.db"))?;

let mut row = HashMap::new();
row.insert("id".to_string(), "doc-1".to_string());
row.insert("title".to_string(), "Transformer Architecture".to_string());
row.insert("body".to_string(), "Relies entirely on self-attention...".to_string());
db.insert("documents", row)?;

let rows = db.query("SELECT * FROM documents")?;`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="sled-api">Sled Rust API</h2>
      <CodeBlock language="rust">{`// requires --features kv-store
use ironvault::rag::{Database, SledDatabase};
use std::collections::HashMap;
use std::path::Path;

let mut db = SledDatabase::new(Path::new("./knowledge_sled"))?;

// Same Database trait — identical interface
let mut row = HashMap::new();
row.insert("id".to_string(), "doc-2".to_string());
row.insert("body".to_string(), "Reinforcement Learning from Human Feedback...".to_string());
db.insert("documents", row)?;`}</CodeBlock>
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
      <CodeBlock language="rust">{`use ironvault::rag::{cosine_similarity, Document, DocumentStore};
use std::collections::HashMap;

let mut store = DocumentStore::new();

store.add_document(Document {
    id: "doc-1".to_string(),
    content: "transformer attention".to_string(),
    metadata: HashMap::new(),
    embedding: Some(vec![0.1, 0.2, 0.3]),
    chunk_info: None,
})?;

// Nearest documents to a query vector
let hits: Vec<(String, f32)> = store.search_similar(&[0.1, 0.2, 0.3], 5);

// Or compare two vectors directly
let score = cosine_similarity(&[0.1, 0.2, 0.3], &[0.2, 0.1, 0.3]);`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="document-schema">Document Schema</h2>
      <CodeBlock language="rust">{`pub struct Document {
    pub id: String,                          // Unique document identifier
    pub content: String,                     // Document text
    pub metadata: HashMap<String, String>,   // Arbitrary string metadata
    pub embedding: Option<Vec<f32>>,         // Optional embedding vector
    pub chunk_info: Option<ChunkInfo>,       // Set when the doc is a chunk
}

pub struct ChunkInfo {
    pub parent_id: Option<String>,  // Parent document ID
    pub chunk_index: usize,         // Index of this chunk
    pub total_chunks: usize,        // Total chunks for the parent
    pub overlap: usize,             // Overlap with adjacent chunks, in characters
}`}</CodeBlock>
    </>
  );
}
