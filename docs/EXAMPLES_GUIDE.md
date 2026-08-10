# 🚀 Quick Start Examples Guide

This guide shows you how to explore IronVault's features using the included examples.

## 📚 Available Examples

IronVault includes 5 comprehensive examples demonstrating all features:

### 1. `basic_usage.rs` - Core Vault Operations ⭐ **START HERE**
**What it shows**: Essential vault operations for everyday use
```bash
cargo run --example basic_usage
```

**Features demonstrated**:
- Creating and unlocking a vault
- Storing models with encryption
- Retrieving models
- Version control and history
- Model metadata management

**Perfect for**: First-time users, learning the basics

---

### 2. `security_demo.rs` - Security Features
**What it shows**: FIPS 140-3 encryption and compliance
```bash
cargo run --example security_demo
```

**Features demonstrated**:
- Military-grade encryption (AES-256-GCM)
- Argon2id key derivation
- Secure key management
- Audit logging
- Compliance checking (FIPS, CMMC, MITRE ATT&CK)

**Perfect for**: Security-conscious users, enterprise deployments

---

### 3. `utilities_demo.rs` - Model Utilities
**What it shows**: 8 powerful utilities for model management
```bash
cargo run --example utilities_demo
```

**Features demonstrated**:
- **Archiving**: Create TAR/ZIP backups
- **Caching**: LRU cache for fast access
- **Deduplication**: Find duplicate models
- **Analysis**: Size, parameters, compression ratios
- **Quantization**: Track FP32→INT8 conversions
- **Pruning**: Monitor sparsity levels
- **Export**: Share models with metadata
- **Compression Analysis**: Predict compression gains

**Perfect for**: Model management, storage optimization

---

### 4. `rag_demo.rs` - RAG & Rule-Based Systems
**What it shows**: Building intelligent systems with RAG
```bash
cargo run --example rag_demo
```

**Features demonstrated**:
- Document store with vector embeddings
- Knowledge base with text chunking
- Semantic search and retrieval
- Rule engine for business logic
- In-memory database operations
- Retrieval caching for performance

**Perfect for**: AI applications, intelligent agents

---

### 5. `mcp_tools_demo.rs` - Model Context Protocol
**What it shows**: MCP tools for AI agent workflows
```bash
cargo run --example mcp_tools_demo
```

**Features demonstrated**:
- Creating custom MCP tools
- MCP server setup and registration
- 4 built-in RAG tools (search, add, chunk, execute)
- Custom tool executors
- Context-aware tool execution
- Complete RAG+MCP pipeline

**Perfect for**: AI agents, automation, tool-based workflows

---

## 🎯 Quick Start Path

### For Beginners (5 minutes)
1. Run `basic_usage.rs` to see core features
2. Read the output to understand vault operations
3. Try modifying the example code

### For Model Management (10 minutes)
1. Run `basic_usage.rs` - Learn the basics
2. Run `utilities_demo.rs` - See all 8 utilities
3. Explore archiving, deduplication, analysis

### For AI Applications (15 minutes)
1. Run `basic_usage.rs` - Core operations
2. Run `rag_demo.rs` - RAG features
3. Run `mcp_tools_demo.rs` - Tool integration

### For Enterprise Security (10 minutes)
1. Run `security_demo.rs` - See encryption in action
2. Run `basic_usage.rs` - Understand versioning
3. Check compliance features

---

## 📖 Learning Path by Feature

### Want to learn: **Encryption & Security**
→ Run: `security_demo.rs`
→ Read: `docs/SECURITY.md`

### Want to learn: **Version Control**
→ Run: `basic_usage.rs`
→ Try: Storing multiple versions

### Want to learn: **Model Utilities**
→ Run: `utilities_demo.rs`
→ Read: `docs/UTILITIES.md`

### Want to learn: **RAG Systems**
→ Run: `rag_demo.rs`
→ Read: `docs/RAG.md`

### Want to learn: **MCP Tools**
→ Run: `mcp_tools_demo.rs`
→ Read: `docs/MCP_TOOLS.md`

---

## 🔥 Real-World Workflows

### Workflow 1: Secure Model Storage
```bash
# 1. See how it works
cargo run --example basic_usage

# 2. Try it yourself with CLI
iv init
iv store my-model ./model.pt --format pytorch
iv get my-model ./retrieved.pt
```

### Workflow 2: Backup & Restore
```bash
# 1. Learn about archiving
cargo run --example utilities_demo

# 2. Create your own backup
iv archive model1 model2 backup.tar
iv extract backup.tar --output ./restored
```

### Workflow 3: Find Duplicates
```bash
# 1. See deduplication in action
cargo run --example utilities_demo

# 2. Find your duplicates
iv deduplicate --detailed
```

### Workflow 4: RAG Application
```bash
# 1. Learn RAG features
cargo run --example rag_demo

# 2. Build your RAG system
# Use the library API shown in the example
```

### Workflow 5: AI Agent with Tools
```bash
# 1. Understand MCP tools
cargo run --example mcp_tools_demo

# 2. Create custom tools
# Follow the patterns in the example
```

---

## 💡 Pro Tips

### Tip 1: Run Examples in Order
Start with `basic_usage.rs`, then move to specialized examples based on your needs.

### Tip 2: Read the Code
All examples are heavily commented. Open them in your editor while running.

### Tip 3: Experiment
Copy example code, modify it, break it, fix it. That's how you learn!

### Tip 4: Check Documentation
Each example references detailed documentation:
- `docs/QUICKSTART.md` - Getting started
- `docs/UTILITIES.md` - Utilities guide
- `docs/RAG.md` - RAG features
- `docs/MCP_TOOLS.md` - MCP tools

### Tip 5: Use the CLI
Examples show library API. CLI commands are simpler for quick tasks.

---

## 📊 Example Comparison

| Example             | Features            | Lines | Run Time | Complexity   |
| ------------------- | ------------------- | ----- | -------- | ------------ |
| `basic_usage.rs`    | 5 core features     | ~250  | ~1s      | ⭐ Easy       |
| `security_demo.rs`  | 6 security features | ~350  | ~2s      | ⭐⭐ Medium    |
| `utilities_demo.rs` | 8 utilities         | ~450  | ~2s      | ⭐⭐ Medium    |
| `rag_demo.rs`       | 6 RAG components    | ~450  | ~1s      | ⭐⭐⭐ Advanced |
| `mcp_tools_demo.rs` | 6 MCP sections      | ~450  | ~1s      | ⭐⭐⭐ Advanced |

**Total**: 5 examples, ~2000 lines of demo code!

---

## 🎓 After the Examples

Once you've run the examples, you're ready to:

1. **Build your own application** using the library API
2. **Use the CLI** for quick model operations
3. **Read the comprehensive docs** for deep dives
4. **Check the test suite** for more usage patterns

---

## 🆘 Need Help?

- **Can't run examples?** Make sure you have Rust installed: `cargo --version`
- **Example not working?** Check the test suite: `cargo test`
- **Want more examples?** Check `tests/*.rs` for 171 test examples
- **Need API docs?** Run: `cargo doc --open`

---

## 🏆 What You'll Learn

By running all 5 examples, you'll understand:

✅ How to store and retrieve models securely  
✅ How version control works for AI models  
✅ How to use 8 powerful utilities  
✅ How to build RAG systems  
✅ How to integrate MCP tools  
✅ How to optimize performance  
✅ Which FIPS-approved algorithms are used, and why that is not FIPS 140-3 validation  

**Time investment**: 30-40 minutes to run all examples  
**Knowledge gain**: Complete understanding of IronVault  

---

**Ready to start? Run your first example:**

```bash
cargo run --example basic_usage
```

🚀 **Let's go!**
