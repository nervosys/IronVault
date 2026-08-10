# Database Support for RAG Knowledge Base

IronVault provides comprehensive database support for building RAG (Retrieval-Augmented Generation) systems with persistent knowledge bases.

## Overview

The database system supports:
- **SQLite**: Full-featured relational database with SQL support
- **Sled**: High-performance embedded key-value store
- **Document storage**: Text, metadata, and embeddings
- **Vector support**: Store and manage embedding vectors
- **Search capabilities**: Full-text search and similarity search

---

## Database Backends

### SQLite Database

**Features**:
- SQL query support
- ACID transactions
- Full-text search
- Relational data model
- Embedding blob storage
- Metadata indexing

**When to use**:
- Need SQL queries
- Complex data relationships
- Full-text search requirements
- Standard database operations

**Performance**:
- Excellent for up to millions of documents
- Efficient indexing
- Low memory footprint
- File-based persistence

### Sled Key-Value Store

**Features**:
- Lightning-fast KV operations
- ACID guarantees
- Zero-copy reads
- Crash recovery
- Prefix scans
- Atomic batch operations

**When to use**:
- High-performance requirements
- Simple key-value operations
- Need fast prefix searches
- Embedded database scenarios

**Performance**:
- Millions of ops/second
- Sub-millisecond latency
- Minimal memory usage
- Optimized for SSD

---

## CLI Commands

### Initialize Database

Create a new database for your RAG system:

```bash
# SQLite database
iv database init --path knowledge.db --db-type sqlite

# Sled database
iv database init --path knowledge --db-type sled
```

### Store Documents

Add documents to your knowledge base:

```bash
# Store a document
iv database store --path knowledge.db --input document.txt

# Store with ID
iv database store --path knowledge.db --input doc.txt --id doc-001

# Store with metadata
iv database store --path knowledge.db --input paper.txt \
    --metadata "category=research" \
    --metadata "author=Smith" \
    --metadata "year=2025"
```

### Retrieve Documents

Get a specific document by ID:

```bash
iv database get --path knowledge.db fccdbda6-b414-4980-bd87-7027763159b1
```

Output:
```
📄 Document Found:
   ID: fccdbda6-b414-4980-bd87-7027763159b1
   Content (63 chars):
   This is a test document about AI models and machine learning.

   Metadata:
     category: ml
     author: test
```

### Search Documents

Full-text search across your knowledge base:

```bash
# Search for documents
iv database search --path knowledge.db "machine learning"

# Limit results
iv database search --path knowledge.db "AI" --limit 5
```

Output:
```
📊 Found 1 document(s):

1. fccdbda6-b414-4980-bd87-7027763159b1 (63)
   This is a test document about AI models and machine learning.
```

### List All Documents

View all documents in the database:

```bash
iv database list --path knowledge.db
```

Output:
```
📊 Total documents: 1
1. fccdbda6-b414-4980-bd87-7027763159b1 (63 chars)
```

### Delete Documents

Remove a document from the database:

```bash
iv database delete --path knowledge.db doc-001
```

### Export Database

Export all documents to JSON:

```bash
iv database export --path knowledge.db --output backup.json
```

### Import Documents

Import documents from JSON:

```bash
iv database import --path knowledge.db --input backup.json
```

### Database Statistics

View database statistics:

```bash
iv database stats --path knowledge.db
```

Output:
```
📊 Database statistics
   Database: knowledge.db

   Documents: 1
   Total characters: 63
   With embeddings: 0
   Average document size: 63 chars
```

---

## Programmatic Usage

### SQLite Backend

```rust
use ironvault::rag::{SQLiteDatabase, Document};
use std::collections::HashMap;
use std::path::Path;

// Create database
let db = SQLiteDatabase::new(Path::new("knowledge.db"))?;

// Create document
let mut metadata = HashMap::new();
metadata.insert("category".to_string(), "ml".to_string());

let doc = Document {
    id: "doc-001".to_string(),
    content: "Machine learning is a subset of AI...".to_string(),
    metadata,
    embedding: None,
    chunk_info: None,
};

// Store document
db.store_document(&doc)?;

// Retrieve document
if let Some(doc) = db.get_document("doc-001")? {
    println!("Found: {}", doc.content);
}

// Search documents
let results = db.search_documents("machine learning", 10)?;
for doc in results {
    println!("Match: {}", doc.id);
}
```

### Sled Backend

```rust
use ironvault::rag::{SledDatabase, Document};
use std::path::Path;

// Create database
let db = SledDatabase::new(Path::new("knowledge"))?;

// Store document
db.store_document(&doc)?;

// Retrieve document
if let Some(doc) = db.get_document("doc-001")? {
    println!("Found: {}", doc.content);
}

// List all documents
let ids = db.list_documents()?;
println!("Total: {} documents", ids.len());

// Search by prefix
let results = db.search_prefix("doc-")?;
```

### Database Trait

Both backends implement a common `Database` trait:

```rust
use ironvault::rag::Database;
use std::collections::HashMap;

// Generic database operations
fn process_database<D: Database>(mut db: D) -> Result<()> {
    // Insert data
    let mut data = HashMap::new();
    data.insert("id".to_string(), "1".to_string());
    data.insert("name".to_string(), "Test".to_string());
    db.insert("table1", data)?;

    // Query data
    let results = db.query("table1 WHERE id=1")?;
    
    // Update data
    let mut updates = HashMap::new();
    updates.insert("name".to_string(), "Updated".to_string());
    db.update("table1", "1", updates)?;

    // Delete data
    db.delete("table1", "1")?;

    Ok(())
}
```

---

## Document Structure

### Document Schema

```rust
pub struct Document {
    /// Unique document identifier
    pub id: String,

    /// Document content/text
    pub content: String,

    /// Document metadata (key-value pairs)
    pub metadata: HashMap<String, String>,

    /// Optional embedding vector (f32 array)
    pub embedding: Option<Vec<f32>>,

    /// Chunk information (if document is split)
    pub chunk_info: Option<ChunkInfo>,
}
```

### Chunk Information

For large documents split into chunks:

```rust
pub struct ChunkInfo {
    /// Parent document ID
    pub parent_id: Option<String>,

    /// Chunk index (0-based)
    pub chunk_index: usize,

    /// Total number of chunks
    pub total_chunks: usize,

    /// Overlap with adjacent chunks (in characters)
    pub overlap: usize,
}
```

---

## Working with Embeddings

### Storing Embeddings

```rust
use ironvault::rag::{SQLiteDatabase, Document};

// Create document with embedding
let embedding = vec![0.1, 0.2, 0.3, 0.4]; // Your embedding vector

let doc = Document {
    id: "doc-001".to_string(),
    content: "Example text".to_string(),
    metadata: HashMap::new(),
    embedding: Some(embedding),
    chunk_info: None,
};

db.store_document(&doc)?;
```

### Similarity Search

```rust
use ironvault::rag::DocumentStore;

// Create document store
let mut store = DocumentStore::new();

// Add documents with embeddings
store.add_document(doc1)?;
store.add_document(doc2)?;
store.add_document(doc3)?;

// Search by similarity
let query_embedding = vec![0.15, 0.25, 0.35, 0.45];
let results = store.search_similar(&query_embedding, 5);

for (doc_id, similarity) in results {
    println!("Document: {} (similarity: {:.4})", doc_id, similarity);
}
```

---

## Document Chunking

### Chunking Strategy

For large documents, split into manageable chunks:

```rust
use ironvault::rag::{Document, ChunkInfo};

fn chunk_document(content: &str, chunk_size: usize, overlap: usize) -> Vec<Document> {
    let mut chunks = Vec::new();
    let total_chunks = (content.len() + chunk_size - 1) / chunk_size;
    let parent_id = uuid::Uuid::new_v4().to_string();

    for (i, chunk_text) in content.as_bytes()
        .chunks(chunk_size)
        .enumerate() 
    {
        let chunk_content = String::from_utf8_lossy(chunk_text).to_string();
        
        let doc = Document {
            id: format!("{}-{}", parent_id, i),
            content: chunk_content,
            metadata: HashMap::new(),
            embedding: None,
            chunk_info: Some(ChunkInfo {
                parent_id: Some(parent_id.clone()),
                chunk_index: i,
                total_chunks,
                overlap,
            }),
        };
        
        chunks.push(doc);
    }

    chunks
}

// Usage
let chunks = chunk_document(large_text, 1000, 100);
for chunk in chunks {
    db.store_document(&chunk)?;
}
```

---

## Best Practices

### 1. Choose the Right Backend

**Use SQLite when**:
- Need SQL queries
- Require full-text search
- Have complex metadata
- Want standard database features

**Use Sled when**:
- Need maximum performance
- Simple key-value operations
- Embedded scenarios
- Prefix-based searches

### 2. Metadata Strategy

Store useful metadata for filtering:

```bash
iv database store --path kb.db --input paper.txt \
    --metadata "type=research" \
    --metadata "domain=ml" \
    --metadata "year=2025" \
    --metadata "authors=Smith,Jones" \
    --metadata "conference=ICML"
```

### 3. Document IDs

Use meaningful IDs for easier management:

```bash
# Good IDs
--id "arxiv-2025-001"
--id "paper-ml-transformers"
--id "doc-chapter-1-intro"

# Avoid generic IDs if possible
--id "fccdbda6-b414-4980-bd87-7027763159b1"
```

### 4. Chunking Large Documents

For documents > 5000 characters, chunk them:

```python
# Python example for chunking
def chunk_text(text, chunk_size=1000, overlap=100):
    chunks = []
    start = 0
    while start < len(text):
        end = start + chunk_size
        chunk = text[start:end]
        chunks.append(chunk)
        start += chunk_size - overlap
    return chunks
```

### 5. Regular Backups

Export your database regularly:

```bash
# Daily backup
iv database export --path knowledge.db --output "backup-$(date +%Y%m%d).json"

# Restore if needed
iv database import --path knowledge-new.db --input backup-20250107.json
```

---

## Performance Tips

### SQLite Optimization

1. **Use transactions for bulk inserts**:
```rust
// Wrap multiple inserts in a transaction
conn.execute("BEGIN TRANSACTION", [])?;
for doc in documents {
    db.store_document(&doc)?;
}
conn.execute("COMMIT", [])?;
```

2. **Create indexes for frequent queries**:
```sql
CREATE INDEX idx_metadata ON documents(metadata);
CREATE INDEX idx_content ON documents(content);
```

3. **Use prepared statements**:
```rust
// Prepared statements are reused automatically
let stmt = conn.prepare("SELECT * FROM documents WHERE id = ?")?;
```

### Sled Optimization

1. **Batch operations**:
```rust
let mut batch = sled::Batch::default();
for doc in documents {
    let key = doc.id.as_bytes();
    let value = serde_json::to_vec(&doc)?;
    batch.insert(key, value);
}
db.apply_batch(batch)?;
```

2. **Use prefix scans efficiently**:
```rust
// Efficient prefix search
for result in db.scan_prefix("category:ml:") {
    // Process results
}
```

---

## Troubleshooting

### Database Not Found

```
Error: StorageError("Failed to open database")
```

**Solution**: Initialize the database first:
```bash
iv database init --path knowledge.db --db-type sqlite
```

### Permission Denied

```
Error: IoError("Permission denied")
```

**Solution**: Check file permissions:
```bash
chmod 644 knowledge.db  # Linux/macOS
icacls knowledge.db /grant Users:F  # Windows
```

### Database Locked

```
Error: StorageError("database is locked")
```

**Solution**: Close other connections or wait:
```rust
// Use timeout
let conn = Connection::open_with_flags(
    path,
    OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
)?;
conn.busy_timeout(Duration::from_secs(5))?;
```

---

## Integration with RAG Systems

### Complete RAG Pipeline

```rust
use ironvault::rag::{SQLiteDatabase, Document, DocumentStore};

// 1. Initialize database
let db = SQLiteDatabase::new(Path::new("rag_kb.db"))?;

// 2. Ingest documents
let documents = load_documents("./data")?;
for doc in documents {
    // Generate embeddings (using your embedding model)
    let embedding = generate_embedding(&doc.content)?;
    
    let doc_with_embedding = Document {
        embedding: Some(embedding),
        ..doc
    };
    
    db.store_document(&doc_with_embedding)?;
}

// 3. Build in-memory index for fast retrieval
let mut store = DocumentStore::new();
let all_docs = db.search_documents("", 100000)?;
for doc in all_docs {
    store.add_document(doc)?;
}

// 4. Query-time retrieval
fn retrieve_context(query: &str, db: &SQLiteDatabase, store: &DocumentStore) -> Result<Vec<Document>> {
    // Generate query embedding
    let query_embedding = generate_embedding(query)?;
    
    // Find similar documents
    let similar = store.search_similar(&query_embedding, 5);
    
    // Retrieve full documents
    let mut contexts = Vec::new();
    for (doc_id, similarity) in similar {
        if let Some(doc) = db.get_document(&doc_id)? {
            contexts.push(doc);
        }
    }
    
    Ok(contexts)
}
```

---

## Migration Between Backends

### SQLite to Sled

```bash
# Export from SQLite
iv database export --path old.db --output data.json

# Import to Sled
iv database init --path new_sled --db-type sled
iv database import --path new_sled --input data.json
```

### Sled to SQLite

```bash
# Export from Sled
iv database export --path old_sled --output data.json

# Import to SQLite
iv database init --path new.db --db-type sqlite
iv database import --path new.db --input data.json
```

---

## Future Enhancements

### Planned Features

- **Vector database integration**: Qdrant, Milvus, LanceDB
- **Advanced indexing**: BM25, semantic search
- **Hybrid search**: Combine keyword and vector search
- **Distributed storage**: Multi-node synchronization
- **Automatic chunking**: Smart document splitting
- **Embedding generation**: Built-in embedding models

---

## Examples

See the `examples/` directory for complete examples:

- `examples/rag_demo.rs` - Complete RAG pipeline
- `examples/database_basic.rs` - Basic database operations
- `examples/similarity_search.rs` - Vector similarity search
- `examples/chunking.rs` - Document chunking strategies

---

## Summary

IronVault provides production-ready database support for RAG systems with:

✅ **SQLite** - Full SQL database with excellent performance  
✅ **Sled** - Lightning-fast embedded KV store  
✅ **Document storage** - Text, metadata, embeddings  
✅ **Search capabilities** - Full-text and similarity  
✅ **CLI tools** - Complete command-line interface  
✅ **Programmatic API** - Easy integration in Rust  

Choose the right backend for your use case and build powerful RAG applications!
