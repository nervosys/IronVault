# Contributing to ironvault

Thank you for your interest in contributing to ironvault! This document provides guidelines and instructions for contributing.

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help maintain a welcoming environment
- Follow the security guidelines

## How to Contribute

### Reporting Bugs

1. Check existing issues to avoid duplicates
2. Use the bug report template
3. Include:
   - OS and version
   - Rust version
   - Steps to reproduce
   - Expected vs actual behavior
   - Error messages/logs

### Suggesting Features

1. Check existing feature requests
2. Describe the use case
3. Explain expected behavior
4. Consider security implications

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Write/update tests
5. Ensure all tests pass (`cargo test`)
6. Run security checks (`cargo audit`, `cargo clippy`)
7. Format code (`cargo fmt`)
8. Commit with clear messages
9. Push to your fork
10. Open a Pull Request
11. **Sign the CLA** — include "I have read the CLA and I agree to its terms" in your first PR description (see [CLA.md](CLA.md))

### Contributor License Agreement

All contributions require a signed Contributor License Agreement (CLA). This
authorizes the maintainer to distribute your contributions under both the
AGPL-3.0-or-later license and the commercial license. See [CLA.md](CLA.md) for
the full agreement.

## Development Setup

### Prerequisites

```bash
# Install Rust (https://rustup.rs/)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
cargo install cargo-audit
cargo install cargo-deny
cargo install cargo-watch
```

### Building

```bash
# Clone repository
git clone https://github.com/nervosys/IronVault.git
cd IronVault

# Build (default features: safetensors, ndarray, sqlite)
cargo build

# Build with all features (all backends + API + GraphQL)
cargo build --features "full,graphql"

# Build optimized release
cargo build --release --features "full,graphql"

# Build with specific features
cargo build --features "api,sqlite,s3"
```

### Testing

```bash
# Run all tests (lib + integration)
cargo test --features "full,graphql"

# Run lib tests only
cargo test --lib --features "full,graphql"

# Run a specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run with coverage (requires cargo-llvm-cov)
cargo llvm-cov --features "full,graphql" --lcov --output-path lcov.info

# Run fuzz targets (requires nightly + cargo-fuzz)
cargo +nightly fuzz run crypto_roundtrip -- -max_total_time=60
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Lint with clippy
cargo clippy -- -D warnings

# Security audit
cargo audit

# Check dependencies
cargo deny check
```

## Code Style

- Follow Rust standard style (`rustfmt`)
- Use meaningful variable/function names
- Add documentation comments (`///`)
- Keep functions focused and small
- Handle errors explicitly
- Use `Result` and `Option` appropriately

## Documentation

- All public APIs must have documentation
- Include examples in doc comments
- Update README.md for user-facing changes
- Add inline comments for complex logic
- Update CHANGELOG.md

## Security Considerations

When contributing, consider:

1. **Cryptography**: 
   - Use approved algorithms only
   - Never implement custom crypto
   - Follow FIPS 140-3 guidelines

2. **Memory Safety**:
   - Use `zeroize` for sensitive data
   - Avoid unsafe code unless necessary
   - Document safety requirements

3. **Input Validation**:
   - Validate all external input
   - Use type-safe APIs
   - Prevent injection attacks

4. **Error Handling**:
   - Don't leak sensitive info in errors
   - Use structured error types
   - Log security events to audit log

## Testing Guidelines

- Write unit tests for new functions
- Add integration tests for features
- Test error conditions
- Test security boundaries
- Use property-based testing where appropriate
- Maintain >80% code coverage

## Commit Messages

Format:
```
<type>(<scope>): <subject>

<body>

<footer>
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance

Example:
```
feat(crypto): add support for additional key derivation functions

Implement support for scrypt as an alternative to Argon2id.
Maintains FIPS 140-3 compliance.

Closes #123
```

## Review Process

1. Maintainer reviews code
2. CI/CD checks must pass
3. Security scan must pass
4. At least one approval required
5. Squash and merge

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create release tag
4. Publish to crates.io
5. Create GitHub release

## Questions?

- Open a discussion on GitHub
- Join our community chat
- Email: dev@nervosys.ai

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
