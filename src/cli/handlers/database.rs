//! Database command handlers for RAG knowledge base operations.

use ironvault::{Result, VaultError};
use std::path::PathBuf;

use crate::cli::args::DatabaseCommands;

pub fn handle_database(command: DatabaseCommands) -> Result<()> {
    #[cfg(not(any(feature = "sqlite", feature = "kv-store")))]
    {
        let _ = command;
        return Err(VaultError::InvalidInput(
            "Database features not enabled. Rebuild with --features sqlite or --features kv-store"
                .to_string(),
        ));
    }

    #[cfg(any(feature = "sqlite", feature = "kv-store"))]
    {
        use ironvault::rag::Document;
        use std::collections::HashMap;

        match command {
            DatabaseCommands::Init { path, db_type } => {
                handle_db_init(path, db_type)?;
            }

            DatabaseCommands::Store {
                path,
                input,
                id,
                metadata,
            } => {
                handle_db_store(path, input, id, metadata)?;
            }

            DatabaseCommands::Get { path, id } => {
                handle_db_get(path, id)?;
            }

            DatabaseCommands::Search { path, query, limit } => {
                handle_db_search(path, query, limit)?;
            }

            DatabaseCommands::List { path } => {
                handle_db_list(path)?;
            }

            DatabaseCommands::Delete { path, id } => {
                handle_db_delete(path, id)?;
            }

            DatabaseCommands::Export { path, output } => {
                handle_db_export(path, output)?;
            }

            DatabaseCommands::Import { path, input } => {
                handle_db_import(path, input)?;
            }

            DatabaseCommands::Stats { path } => {
                handle_db_stats(path)?;
            }

            DatabaseCommands::BuildIndex { path, output } => {
                handle_db_build_index(path, output)?;
            }

            DatabaseCommands::VectorSearch {
                index,
                query,
                limit,
            } => {
                handle_db_vector_search(index, query, limit)?;
            }
        }

        #[allow(clippy::needless_return)]
        return Ok(());

        // --- Sub-handler functions ---

        fn handle_db_init(path: PathBuf, db_type: String) -> Result<()> {
            println!("🗄️  Initializing database");
            println!("   Path: {}", path.display());
            println!("   Type: {}", db_type);

            match db_type.to_lowercase().as_str() {
                #[cfg(feature = "sqlite")]
                "sqlite" => {
                    use ironvault::rag::SQLiteDatabase;
                    let _db = SQLiteDatabase::new(&path)?;
                    let conn = rusqlite::Connection::open(&path).map_err(|e| {
                        VaultError::StorageError(format!("Failed to open database: {}", e))
                    })?;
                    conn.execute(
                        "CREATE TABLE IF NOT EXISTS documents (
                            id TEXT PRIMARY KEY,
                            content TEXT NOT NULL,
                            metadata TEXT,
                            embedding BLOB,
                            chunk_parent_id TEXT,
                            chunk_index INTEGER,
                            chunk_total INTEGER,
                            chunk_overlap INTEGER,
                            created_at TEXT DEFAULT CURRENT_TIMESTAMP
                        )",
                        [],
                    )
                    .map_err(|e| {
                        VaultError::StorageError(format!("Failed to create table: {}", e))
                    })?;
                    println!("✅ SQLite database initialized successfully!");
                }
                #[cfg(feature = "kv-store")]
                "sled" => {
                    use ironvault::rag::SledDatabase;
                    let _db = SledDatabase::new(&path)?;
                    println!("✅ Sled database initialized successfully!");
                }
                _ => {
                    return Err(VaultError::InvalidInput(format!(
                        "Unknown database type: {}. Use 'sqlite' or 'sled'",
                        db_type
                    )));
                }
            }
            Ok(())
        }

        fn handle_db_store(
            path: PathBuf,
            input: PathBuf,
            id: Option<String>,
            metadata: Vec<String>,
        ) -> Result<()> {
            println!("📝 Storing document");
            println!("   Database: {}", path.display());
            println!("   Input: {}", input.display());

            let content = std::fs::read_to_string(&input)?;
            let doc_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let mut meta_map = HashMap::new();
            for meta_str in metadata {
                if let Some((key, value)) = meta_str.split_once('=') {
                    meta_map.insert(key.to_string(), value.to_string());
                }
            }

            let doc = Document {
                id: doc_id.clone(),
                content,
                metadata: meta_map,
                embedding: None,
                chunk_info: None,
            };

            if path.extension().and_then(|s| s.to_str()) == Some("db")
                || path.to_str().unwrap_or("").contains("sqlite")
            {
                #[cfg(feature = "sqlite")]
                {
                    use ironvault::rag::SQLiteDatabase;
                    let db = SQLiteDatabase::new(&path)?;
                    db.store_document(&doc)?;
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    return Err(VaultError::InvalidInput(
                        "SQLite support not enabled".to_string(),
                    ));
                }
            } else {
                #[cfg(feature = "kv-store")]
                {
                    use ironvault::rag::SledDatabase;
                    let db = SledDatabase::new(&path)?;
                    db.store_document(&doc)?;
                }
                #[cfg(not(feature = "kv-store"))]
                {
                    return Err(VaultError::InvalidInput(
                        "Sled support not enabled".to_string(),
                    ));
                }
            }

            println!("✅ Document stored successfully!");
            println!("   ID: {}", doc_id);
            Ok(())
        }

        fn handle_db_get(path: PathBuf, id: String) -> Result<()> {
            println!("🔍 Retrieving document");
            println!("   Database: {}", path.display());
            println!("   ID: {}", id);

            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                #[cfg(feature = "sqlite")]
                {
                    use ironvault::rag::SQLiteDatabase;
                    let db = SQLiteDatabase::new(&path)?;
                    if let Some(doc) = db.get_document(&id)? {
                        println!("\n📄 Document Found:");
                        println!("   ID: {}", doc.id);
                        println!("   Content ({} chars):", doc.content.len());
                        println!("   {}", doc.content);
                        if !doc.metadata.is_empty() {
                            println!("\n   Metadata:");
                            for (key, value) in &doc.metadata {
                                println!("     {}: {}", key, value);
                            }
                        }
                    } else {
                        return Err(VaultError::NotFound(format!("document {id:?}")));
                    }
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    return Err(VaultError::InvalidInput(
                        "SQLite support not enabled".to_string(),
                    ));
                }
            } else {
                #[cfg(feature = "kv-store")]
                {
                    use ironvault::rag::SledDatabase;
                    let db = SledDatabase::new(&path)?;
                    if let Some(doc) = db.get_document(&id)? {
                        println!("\n📄 Document Found:");
                        println!("   ID: {}", doc.id);
                        println!("   Content: {}", doc.content);
                    } else {
                        return Err(VaultError::NotFound(format!("document {id:?}")));
                    }
                }
                #[cfg(not(feature = "kv-store"))]
                {
                    return Err(VaultError::InvalidInput(
                        "Sled support not enabled".to_string(),
                    ));
                }
            }
            Ok(())
        }

        fn handle_db_search(path: PathBuf, query: String, limit: usize) -> Result<()> {
            println!("🔍 Searching documents");
            println!("   Database: {}", path.display());
            println!("   Query: {}", query);

            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                #[cfg(feature = "sqlite")]
                {
                    use ironvault::rag::SQLiteDatabase;
                    let db = SQLiteDatabase::new(&path)?;
                    let results = db.search_documents(&query, limit)?;

                    println!("\n📊 Found {} document(s):", results.len());
                    for (i, doc) in results.iter().enumerate() {
                        println!("\n{}. {} ({})", i + 1, doc.id, doc.content.len());
                        let preview = if doc.content.len() > 100 {
                            format!("{}...", &doc.content[..100])
                        } else {
                            doc.content.clone()
                        };
                        println!("   {}", preview);
                    }
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    let _ = (query, limit);
                    return Err(VaultError::InvalidInput(
                        "SQLite support not enabled".to_string(),
                    ));
                }
            }
            Ok(())
        }

        fn handle_db_list(path: PathBuf) -> Result<()> {
            println!("📋 Listing documents");
            println!("   Database: {}", path.display());

            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                #[cfg(feature = "sqlite")]
                {
                    use ironvault::rag::SQLiteDatabase;
                    let db = SQLiteDatabase::new(&path)?;
                    let results = db.search_documents("", 1000)?;

                    println!("\n📊 Total documents: {}", results.len());
                    for (i, doc) in results.iter().enumerate() {
                        println!("{}. {} ({} chars)", i + 1, doc.id, doc.content.len());
                    }
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    return Err(VaultError::InvalidInput(
                        "SQLite support not enabled".to_string(),
                    ));
                }
            } else {
                #[cfg(feature = "kv-store")]
                {
                    use ironvault::rag::SledDatabase;
                    let db = SledDatabase::new(&path)?;
                    let ids = db.list_documents()?;

                    println!("\n📊 Total documents: {}", ids.len());
                    for (i, id) in ids.iter().enumerate() {
                        println!("{}. {}", i + 1, id);
                    }
                }
                #[cfg(not(feature = "kv-store"))]
                {
                    return Err(VaultError::InvalidInput(
                        "Sled support not enabled".to_string(),
                    ));
                }
            }
            Ok(())
        }

        fn handle_db_delete(path: PathBuf, id: String) -> Result<()> {
            println!("🗑️  Deleting document");
            println!("   Database: {}", path.display());
            println!("   ID: {}", id);

            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                #[cfg(feature = "sqlite")]
                {
                    use ironvault::rag::{Database, SQLiteDatabase};
                    let mut db = SQLiteDatabase::new(&path)?;
                    db.delete("documents", &id)?;
                    println!("✅ Document deleted successfully!");
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    return Err(VaultError::InvalidInput(
                        "SQLite support not enabled".to_string(),
                    ));
                }
            } else {
                #[cfg(feature = "kv-store")]
                {
                    use ironvault::rag::{Database, SledDatabase};
                    let mut db = SledDatabase::new(&path)?;
                    db.delete("", &id)?;
                    println!("✅ Document deleted successfully!");
                }
                #[cfg(not(feature = "kv-store"))]
                {
                    return Err(VaultError::InvalidInput(
                        "Sled support not enabled".to_string(),
                    ));
                }
            }
            Ok(())
        }

        fn handle_db_export(path: PathBuf, output: PathBuf) -> Result<()> {
            println!("📤 Exporting database");
            println!("   Database: {}", path.display());
            println!("   Output: {}", output.display());

            let mut documents = Vec::new();

            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                #[cfg(feature = "sqlite")]
                {
                    use ironvault::rag::SQLiteDatabase;
                    let db = SQLiteDatabase::new(&path)?;
                    documents = db.search_documents("", 100000)?;
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    return Err(VaultError::InvalidInput(
                        "SQLite support not enabled".to_string(),
                    ));
                }
            }

            let json = serde_json::to_string_pretty(&documents)?;
            std::fs::write(&output, json)?;

            println!("✅ Exported {} documents successfully!", documents.len());
            Ok(())
        }

        fn handle_db_import(path: PathBuf, input: PathBuf) -> Result<()> {
            println!("📥 Importing documents");
            println!("   Database: {}", path.display());
            println!("   Input: {}", input.display());

            let json_content = std::fs::read_to_string(&input)?;
            let documents: Vec<Document> = serde_json::from_str(&json_content)?;

            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                #[cfg(feature = "sqlite")]
                {
                    use ironvault::rag::SQLiteDatabase;
                    let db = SQLiteDatabase::new(&path)?;
                    for doc in &documents {
                        db.store_document(doc)?;
                    }
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    return Err(VaultError::InvalidInput(
                        "SQLite support not enabled".to_string(),
                    ));
                }
            } else {
                #[cfg(feature = "kv-store")]
                {
                    use ironvault::rag::SledDatabase;
                    let db = SledDatabase::new(&path)?;
                    for doc in &documents {
                        db.store_document(doc)?;
                    }
                }
                #[cfg(not(feature = "kv-store"))]
                {
                    return Err(VaultError::InvalidInput(
                        "Sled support not enabled".to_string(),
                    ));
                }
            }

            println!("✅ Imported {} documents successfully!", documents.len());
            Ok(())
        }

        fn handle_db_stats(path: PathBuf) -> Result<()> {
            println!("📊 Database statistics");
            println!("   Database: {}", path.display());

            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                #[cfg(feature = "sqlite")]
                {
                    use ironvault::rag::SQLiteDatabase;
                    let db = SQLiteDatabase::new(&path)?;
                    let all_docs = db.search_documents("", 100000)?;

                    let total_docs = all_docs.len();
                    let total_chars: usize = all_docs.iter().map(|d| d.content.len()).sum();
                    let with_embeddings = all_docs.iter().filter(|d| d.embedding.is_some()).count();

                    println!("\n   Documents: {}", total_docs);
                    println!("   Total characters: {}", total_chars);
                    println!("   With embeddings: {}", with_embeddings);
                    println!(
                        "   Average document size: {} chars",
                        total_chars.checked_div(total_docs).unwrap_or(0)
                    );
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    return Err(VaultError::InvalidInput(
                        "SQLite support not enabled".to_string(),
                    ));
                }
            } else {
                #[cfg(feature = "kv-store")]
                {
                    use ironvault::rag::SledDatabase;
                    let db = SledDatabase::new(&path)?;
                    let ids = db.list_documents()?;
                    println!("\n   Documents: {}", ids.len());
                }
                #[cfg(not(feature = "kv-store"))]
                {
                    return Err(VaultError::InvalidInput(
                        "Sled support not enabled".to_string(),
                    ));
                }
            }
            Ok(())
        }

        fn handle_db_build_index(path: PathBuf, output: PathBuf) -> Result<()> {
            println!("🔨 Building vector index");
            println!("   Database: {}", path.display());
            println!("   Output: {}", output.display());

            use ironvault::rag::{SimpleVectorStore, VectorStore};

            let all_docs = if path.extension().and_then(|s| s.to_str()) == Some("db") {
                #[cfg(feature = "sqlite")]
                {
                    use ironvault::rag::SQLiteDatabase;
                    let db = SQLiteDatabase::new(&path)?;
                    db.search_documents("", 100000)?
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    return Err(VaultError::InvalidInput(
                        "SQLite support not enabled".to_string(),
                    ));
                }
            } else {
                #[cfg(feature = "kv-store")]
                {
                    use ironvault::rag::SledDatabase;
                    let db = SledDatabase::new(&path)?;
                    let ids = db.list_documents()?;
                    let mut docs = Vec::new();
                    for id in ids {
                        if let Some(doc) = db.get_document(&id)? {
                            docs.push(doc);
                        }
                    }
                    docs
                }
                #[cfg(not(feature = "kv-store"))]
                {
                    return Err(VaultError::InvalidInput(
                        "Sled support not enabled".to_string(),
                    ));
                }
            };

            let docs_with_embeddings: Vec<_> = all_docs
                .into_iter()
                .filter(|d| d.embedding.is_some())
                .collect();

            if docs_with_embeddings.is_empty() {
                // No index was built. Exiting 0 told the caller one exists, and
                // the next search against it would be the thing that failed.
                println!("⚠️  No documents with embeddings found");
                return Err(VaultError::InvalidInput(
                    "no documents have embeddings, so no index was built — \
                     add embeddings to documents first"
                        .to_string(),
                ));
            }

            let mut store = SimpleVectorStore::new();
            for doc in &docs_with_embeddings {
                store.store_with_embedding(doc)?;
            }

            let index_data = serde_json::to_string_pretty(&docs_with_embeddings)?;
            std::fs::write(&output, index_data)?;

            println!("✅ Vector index built successfully!");
            println!("   Documents indexed: {}", docs_with_embeddings.len());
            println!("   Index size: {} bytes", std::fs::metadata(&output)?.len());
            Ok(())
        }

        fn handle_db_vector_search(index: PathBuf, query: PathBuf, limit: usize) -> Result<()> {
            println!("🔍 Vector similarity search");
            println!("   Index: {}", index.display());
            println!("   Query: {}", query.display());

            use ironvault::rag::{SimpleVectorStore, VectorStore};

            let index_data = std::fs::read_to_string(&index)?;
            let documents: Vec<ironvault::rag::Document> = serde_json::from_str(&index_data)?;

            let mut store = SimpleVectorStore::new();
            for doc in &documents {
                store.store_with_embedding(doc)?;
            }

            let query_data = std::fs::read_to_string(&query)?;
            let query_embedding: Vec<f32> = serde_json::from_str(&query_data)?;

            let results = store.search_similar(&query_embedding, limit)?;

            println!("\n📊 Found {} similar document(s):", results.len());
            for (i, (doc_id, similarity)) in results.iter().enumerate() {
                if let Some(doc) = documents.iter().find(|d| d.id == *doc_id) {
                    println!("\n{}. {} (similarity: {:.4})", i + 1, doc_id, similarity);
                    let preview = if doc.content.len() > 200 {
                        format!("{}...", &doc.content[..200])
                    } else {
                        doc.content.clone()
                    };
                    println!("   {}", preview);
                }
            }
            Ok(())
        }
    }
}
