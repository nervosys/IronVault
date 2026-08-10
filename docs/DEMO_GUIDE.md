# Demo Scripts - Quick Reference

IronVault includes interactive demonstration scripts in both PowerShell and Bash.

## 📋 Available Scripts

### PowerShell (Windows)
```powershell
.\demo.ps1 [options]
```

### Bash (Linux/macOS/WSL)
```bash
./demo.sh [options]
```

## 🚀 Quick Start

### Windows (PowerShell)
```powershell
# Quick 2-minute demo
.\demo.ps1 -Quick

# Full demonstration suite (~10 minutes)
.\demo.ps1 -Full

# Specific feature demos
.\demo.ps1 -HuggingFace
.\demo.ps1 -Security
.\demo.ps1 -Utilities
.\demo.ps1 -RAG
```

### Linux/macOS (Bash)
```bash
# Make executable (first time only)
chmod +x demo.sh

# Quick 2-minute demo
./demo.sh --quick

# Full demonstration suite (~10 minutes)
./demo.sh --full

# Specific feature demos
./demo.sh --huggingface
./demo.sh --security
./demo.sh --utilities
./demo.sh --rag
```

## 📖 Demo Options

| Option          | Duration | Description                                        |
| --------------- | -------- | -------------------------------------------------- |
| **Quick**       | ~2 min   | Basic vault operations (store, retrieve, versions) |
| **Full**        | ~10 min  | All examples + security audit + tests              |
| **HuggingFace** | ~3 min   | HuggingFace model integration demo                 |
| **Security**    | ~2 min   | Encryption, compliance, audit logging              |
| **Utilities**   | ~2 min   | Model analysis, compression, deduplication         |
| **RAG**         | ~2 min   | Document store, knowledge base, MCP tools          |

## 🎯 What Each Demo Shows

### Quick Demo (basic_usage)
```
✓ Creating and configuring vault
✓ Unlocking with passphrase
✓ Storing models with metadata
✓ Version control (v1, v2, v3)
✓ Retrieving specific versions
✓ Lineage tracking
✓ Vault statistics
✓ Deleting versions
```

### HuggingFace Demo
```
✓ Simulating HuggingFace model download
✓ Storing 10MB model
✓ Compression (99.6% on synthetic data)
✓ Creating fine-tuned version
✓ Creating quantized version
✓ Version history and lineage tree
✓ Data integrity verification
✓ Vault statistics
```

### Security Demo
```
✓ FIPS 140-3 encryption (AES-256-GCM)
✓ Argon2id key derivation
✓ BLAKE3 checksums
✓ Compliance checks (CMMC, MITRE)
✓ Audit logging
✓ Key management
```

### Utilities Demo
```
✓ Model format detection
✓ Size and parameter analysis
✓ Compression analysis
✓ Deduplication detection
✓ Archive creation (TAR/ZIP)
✓ Model export with metadata
✓ Quantization tracking
```

### RAG Demo
```
✓ Document store creation
✓ Knowledge base with chunking
✓ Semantic search (planned)
✓ MCP tool integration
✓ Rule-based systems
✓ Context retrieval
```

### Full Demo
Runs all of the above plus:
```
✓ Security audit (cargo audit)
✓ Complete test suite (37 tests)
✓ Performance benchmarks
✓ Comprehensive output
```

## 💡 Example Usage

### First Time Setup
```powershell
# Windows
git clone https://github.com/yourusername/ironvault
cd ironvault
.\demo.ps1 -Quick
```

```bash
# Linux/macOS
git clone https://github.com/yourusername/ironvault
cd ironvault
chmod +x demo.sh
./demo.sh --quick
```

### Running Specific Demos

#### Show HuggingFace Integration
```powershell
# Windows
.\demo.ps1 -HuggingFace

# Linux/macOS
./demo.sh --huggingface
```

#### Verify Security Compliance
```powershell
# Windows
.\demo.ps1 -Security

# Linux/macOS
./demo.sh --security
```

#### Test All Features
```powershell
# Windows (takes ~10 minutes)
.\demo.ps1 -Full

# Linux/macOS
./demo.sh --full
```

## 🎨 Script Features

### Colored Output
- ✅ Green checkmarks for success
- ℹ️ Blue arrows for information
- ⚠️ Yellow exclamation for warnings
- ❌ Red X for errors

### Automatic Build
- Compiles project in release mode (optimized)
- Checks for Rust/Cargo installation
- Shows build progress

### Error Handling
- Exits on build failures
- Reports test failures
- Validates prerequisites

### Progress Tracking
- Shows current step
- Displays duration
- Provides status updates

## 📊 Expected Output

### Successful Quick Demo
```
╔═══════════════════════════════════════════════════╗
║     IronVault - Demonstration Script        ║
╚═══════════════════════════════════════════════════╝

=== Prerequisites Check ===
✓ Cargo found: cargo 1.70.0+

=== Building IronVault ===
→ Building release version (optimized)...
✓ Build completed successfully!

=== Running: basic_usage ===
→ Basic vault operations: store, retrieve, version control

=== IronVault Basic Example ===

1. Creating vault...
   ✓ Vault created at: ~/.local/share/ironvault/vaults

2. Unlocking vault...
   ✓ Vault unlocked

[... demo output continues ...]

✓ Example 'basic_usage' completed successfully!

╔═══════════════════════════════════════════════════╗
║           Demonstration Complete                  ║
╚═══════════════════════════════════════════════════╝

Next steps:
  • Read the documentation: README.md
  • View security status: SECURITY_STATUS.md
  • Check feature demo: FEATURES_DEMO.md
```

## 🔧 Troubleshooting

### "Cargo not found"
**Solution**: Install Rust from https://rustup.rs/

### "Build failed"
**Solutions**:
1. Update Rust: `rustup update`
2. Clean build: `cargo clean && cargo build --release`
3. Check dependencies: `cargo check`

### "Permission denied" (Linux/macOS)
**Solution**: Make script executable: `chmod +x demo.sh`

### "Execution policy" error (Windows)
**Solution**: 
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

## 🎯 Performance Notes

### Build Times
- First build: 5-10 minutes (downloads dependencies)
- Subsequent builds: 30-60 seconds (cached)
- Release mode: Optimized for performance

### Demo Durations
- Quick: ~2 minutes
- Individual: ~2-3 minutes each
- Full suite: ~10 minutes

### Disk Space
- Source code: ~5 MB
- Dependencies: ~500 MB (first build)
- Build artifacts: ~100 MB
- Demo outputs: Minimal (cleaned up automatically)

## 📚 Related Documentation

- **[README.md](https://github.com/nervosys/IronVault/blob/master/README.md)** - Complete project documentation
- **[FEATURES_DEMO.md](https://github.com/nervosys/IronVault/blob/master/reports/FEATURES_DEMO.md)** - Detailed feature showcase
- **[QUICKSTART.md](QUICKSTART.md)** - 5-minute setup guide
- **[PRODUCTION_READY.md](https://github.com/nervosys/IronVault/blob/master/reports/PRODUCTION_READY.md)** - Production deployment guide
- **[examples/](https://github.com/nervosys/IronVault/blob/master/examples/)** - Source code for all demos

## 🤝 Contributing

Found an issue with the demo scripts? Please:
1. Check [CONTRIBUTING.md](https://github.com/nervosys/IronVault/blob/master/CONTRIBUTING.md) for guidelines
2. Open an issue on GitHub
3. Submit a pull request with improvements

## 📝 License

Demo scripts are part of IronVault and licensed under AGPL-3.0-or-later.
See [LICENSE](https://github.com/nervosys/IronVault/blob/master/LICENSE) for details.

---

**Ready to see IronVault in action?**

```powershell
# Windows
.\demo.ps1 -Quick

# Linux/macOS
./demo.sh --quick
```

**Questions?** Check out the [README.md](https://github.com/nervosys/IronVault/blob/master/README.md) or open an issue!
