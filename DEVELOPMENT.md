# Getting Started with ironvault Development

This guide will help you get started with developing and contributing to ironvault.

## Prerequisites

### Required

- **Rust** 1.75 or later ([Install](https://rustup.rs/))
- **Git** for version control
- **Cargo** (comes with Rust)

### Recommended

- **Visual Studio Code** with rust-analyzer extension
- **cargo-edit** for dependency management: `cargo install cargo-edit`
- **cargo-watch** for auto-rebuild: `cargo install cargo-watch`
- **cargo-audit** for security: `cargo install cargo-audit`
- **cargo-deny** for policy checking: `cargo install cargo-deny`

### Platform-Specific

#### Windows
- Visual Studio Build Tools or MinGW-w64

#### Linux
- Build essentials: `sudo apt install build-essential pkg-config`

#### macOS
- Xcode Command Line Tools: `xcode-select --install`

## Quick Setup

```bash
# Clone repository
git clone https://github.com/nervosys/IronVault.git
cd IronVault

# Build project
cargo build

# Run tests
cargo test

# Run example
cargo run --example basic_usage
```

## Development Workflow

### 1. Build the Project

```bash
# Debug build (fast compilation, slower runtime)
cargo build

# Release build (slower compilation, fast runtime)
cargo build --release

# Build with all optional features
cargo build --all-features

# Clean build artifacts
cargo clean
```

### 2. Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_vault_creation

# Run integration tests only
cargo test --test integration_tests

# Run with coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### 3. Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Lint with clippy
cargo clippy

# Clippy with strict warnings
cargo clippy -- -D warnings

# Check without building
cargo check
```

### 4. Security Checks

```bash
# Audit dependencies for vulnerabilities
cargo audit

# Check dependency licenses and policies
cargo deny check

# Both in one command
cargo audit && cargo deny check
```

### 5. Documentation

```bash
# Generate and open documentation
cargo doc --open

# Generate docs for all features
cargo doc --all-features --open

# Check doc tests
cargo test --doc
```

### 6. Running Examples

```bash
# Basic usage example
cargo run --example basic_usage

# Security demo
cargo run --example security_demo
```

### 7. Using the CLI

```bash
# Build and install locally
cargo install --path .

# Run without installing
cargo run -- init

# Run with arguments
cargo run -- store my-model ./model.pt --format pytorch

# Get help
cargo run -- --help
```

## Project Structure Guide

```
src/
├── lib.rs              # Library entry point, public API
├── main.rs             # CLI application entry point
├── error.rs            # Error types and Result aliases
├── config.rs           # Configuration and XDG paths
├── vault.rs            # Main vault implementation
├── storage.rs          # Encrypted storage backend
├── version.rs          # Version control system
├── formats.rs          # Model format handling
├── audit.rs            # Security audit logging
├── compliance.rs       # Compliance verification
└── crypto/
    ├── mod.rs          # Crypto module (FIPS crypto)
    └── compression.rs  # Compression algorithms
```

### Key Files to Understand

1. **`src/vault.rs`**: Main vault logic - start here
2. **`src/crypto/mod.rs`**: Encryption implementation
3. **`src/version.rs`**: Version control system
4. **`src/storage.rs`**: How data is stored on disk
5. **`src/main.rs`**: CLI implementation

## Common Development Tasks

### Adding a New Feature

1. Create a feature branch:
   ```bash
   git checkout -b feature/my-new-feature
   ```

2. Implement the feature with tests:
   ```rust
   // In src/my_feature.rs
   pub fn my_feature() -> Result<()> {
       // Implementation
       Ok(())
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_my_feature() {
           assert!(my_feature().is_ok());
       }
   }
   ```

3. Add documentation:
   ```rust
   /// Does something amazing
   ///
   /// # Arguments
   ///
   /// * `arg` - Description
   ///
   /// # Examples
   ///
   /// ```
   /// use ironvault::my_feature;
   /// my_feature();
   /// ```
   pub fn my_feature() { }
   ```

4. Run checks:
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   ```

5. Commit and push:
   ```bash
   git add .
   git commit -m "feat: add my new feature"
   git push origin feature/my-new-feature
   ```

### Adding a New Model Format

1. Add to `ModelFormat` enum in `src/formats.rs`:
   ```rust
   pub enum ModelFormat {
       // ...existing formats...
       MyNewFormat,
   }
   ```

2. Update format detection:
   ```rust
   pub fn from_extension(ext: &str) -> Self {
       match ext.to_lowercase().as_str() {
           // ...existing matches...
           "mynew" => ModelFormat::MyNewFormat,
           _ => ModelFormat::Custom(ext.to_string()),
       }
   }
   ```

3. Add tests:
   ```rust
   #[test]
   fn test_new_format() {
       assert_eq!(
           ModelFormat::from_extension("mynew"),
           ModelFormat::MyNewFormat
       );
   }
   ```

### Debugging

#### Enable Logging

```bash
# Set log level
export RUST_LOG=debug
cargo run -- list

# Specific module
export RUST_LOG=ironvault::vault=trace
cargo run -- store model ./model.pt
```

#### Using Debugger

**VS Code**: Create `.vscode/launch.json`:
```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug ironvault",
            "cargo": {
                "args": ["build", "--bin=ironvault"],
                "filter": {
                    "name": "ironvault",
                    "kind": "bin"
                }
            },
            "args": ["init"],
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

**GDB** (Linux):
```bash
cargo build
gdb target/debug/ironvault
(gdb) run init
```

**LLDB** (macOS):
```bash
cargo build
lldb target/debug/ironvault
(lldb) run init
```

## Testing Strategies

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        assert_eq!(2 + 2, 4);
    }
}
```

### Integration Tests
In `tests/integration_tests.rs`:
```rust
use ironvault::Vault;

#[test]
fn test_vault_workflow() {
    let vault = Vault::new(None).unwrap();
    // Test complete workflow
}
```

### Property-Based Tests
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encryption_roundtrip(data: Vec<u8>) {
        let encrypted = encrypt(&data)?;
        let decrypted = decrypt(&encrypted)?;
        assert_eq!(data, decrypted);
    }
}
```

## Performance Profiling

### Benchmarking
```bash
# Run benchmarks
cargo bench

# Specific benchmark
cargo bench crypto_bench
```

### CPU Profiling
```bash
# Install flamegraph
cargo install flamegraph

# Profile
cargo flamegraph --bin ironvault -- store model ./large_model.pt
```

### Memory Profiling
```bash
# Use valgrind (Linux)
valgrind --leak-check=full target/debug/iv init

# Use heaptrack (Linux)
heaptrack target/debug/iv init
```

## Continuous Integration

The project uses GitHub Actions for CI/CD:

- **`.github/workflows/ci.yml`**: Runs on every push/PR
  - Builds on Linux, Windows, macOS
  - Runs tests with multiple Rust versions
  - Performs security audits
  - Checks code formatting and linting

- **`.github/workflows/release.yml`**: Runs on version tags
  - Builds release binaries
  - Publishes to crates.io
  - Creates GitHub releases

## Troubleshooting

### Build Failures

**Problem**: Compilation errors
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build
```

**Problem**: Missing dependencies
```bash
# Update dependencies
cargo update
```

### Test Failures

**Problem**: Tests fail locally
```bash
# Run verbose
cargo test -- --nocapture --test-threads=1

# Check for file permissions issues
chmod 700 ~/.local/share/ironvault
```

### Clippy Warnings

```bash
# Fix automatically when possible
cargo clippy --fix

# Allow specific warnings temporarily
#[allow(clippy::warning_name)]
```

## Best Practices

### Security

1. **Never log sensitive data** (passphrases, keys)
2. **Use `zeroize`** for sensitive memory
3. **Validate all inputs** before processing
4. **Check crypto operation results**
5. **Follow FIPS 140-3 guidelines**

### Code Style

1. Run `cargo fmt` before committing
2. Fix all clippy warnings
3. Add documentation to public APIs
4. Write tests for new features
5. Keep functions small and focused

### Git Workflow

1. Create feature branches
2. Write descriptive commit messages
3. Squash commits before merging
4. Update CHANGELOG.md
5. Reference issues in commits

## Resources

### Rust Resources
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rustlings](https://github.com/rust-lang/rustlings)

### Cryptography
- [FIPS 140-3 Standard](https://csrc.nist.gov/publications/detail/fips/140/3/final)
- [NIST Cryptographic Standards](https://csrc.nist.gov/projects/cryptographic-standards-and-guidelines)
- [RustCrypto](https://github.com/RustCrypto)

### Security
- [MITRE ATT&CK](https://attack.mitre.org/)
- [CMMC 2.0](https://www.acq.osd.mil/cmmc/)
- [CVE Database](https://cve.mitre.org/)

## Getting Help

- **Documentation**: https://docs.rs/ironvault
- **Discussions**: https://github.com/nervosys/IronVault/discussions
- **Issues**: https://github.com/nervosys/IronVault/issues
- **Email**: dev@nervosys.ai

## Next Steps

1. Read the [Architecture Overview](IMPLEMENTATION.md)
2. Browse the [API Documentation](https://docs.rs/ironvault)
3. Try the [Examples](examples/)
4. Check [Open Issues](https://github.com/nervosys/IronVault/issues)
5. Join the [Discussions](https://github.com/nervosys/IronVault/discussions)

Happy coding! 🦀
