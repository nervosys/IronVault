#!/usr/bin/env bash
# Build script for IronVault on Unix systems

set -e

COMMAND="${1:-build}"
RELEASE=false
VERBOSE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            RELEASE=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        *)
            COMMAND="$1"
            shift
            ;;
    esac
done

header() {
    echo ""
    echo "=== $1 ==="
}

build() {
    header "Building IronVault"
    if [ "$RELEASE" = true ]; then
        cargo build --release
    else
        cargo build
    fi
}

test() {
    header "Running Tests"
    if [ "$VERBOSE" = true ]; then
        cargo test --all-features -- --nocapture
    else
        cargo test --all-features
    fi
}

clean() {
    header "Cleaning Build Artifacts"
    cargo clean
    rm -rf target/
}

install() {
    header "Installing IronVault"
    cargo install --path .
}

format() {
    header "Formatting Code"
    cargo fmt
}

lint() {
    header "Running Linters"
    cargo fmt -- --check
    cargo clippy -- -D warnings
}

security() {
    header "Running Security Checks"
    
    echo "Installing cargo-audit..."
    cargo install cargo-audit --quiet || true
    
    echo "Running cargo audit..."
    cargo audit
    
    echo "Installing cargo-deny..."
    cargo install cargo-deny --quiet || true
    
    echo "Running cargo deny..."
    cargo deny check
}

documentation() {
    header "Generating Documentation"
    cargo doc --no-deps --open
}

release_build() {
    header "Building Release"
    format
    lint
    security
    test
    cargo build --release --all-features
    
    echo ""
    echo "✓ Release build complete!"
    echo "Binary location: target/release/iv"
}

all() {
    format
    lint
    test
    build
}

# Execute command
case "$COMMAND" in
    build)
        build
        ;;
    test)
        test
        ;;
    clean)
        clean
        ;;
    install)
        install
        ;;
    format)
        format
        ;;
    lint)
        lint
        ;;
    security)
        security
        ;;
    doc)
        documentation
        ;;
    release)
        release_build
        ;;
    all)
        all
        ;;
    *)
        echo "Unknown command: $COMMAND"
        echo ""
        echo "Usage: $0 [command] [options]"
        echo ""
        echo "Commands:"
        echo "  build      - Build the project"
        echo "  test       - Run tests"
        echo "  clean      - Clean build artifacts"
        echo "  install    - Install iv (IronVault)"
        echo "  format     - Format code"
        echo "  lint       - Run linters"
        echo "  security   - Run security checks"
        echo "  doc        - Generate documentation"
        echo "  release    - Build release version"
        echo "  all        - Run format, lint, test, and build"
        echo ""
        echo "Options:"
        echo "  --release  - Build in release mode"
        echo "  --verbose  - Verbose output"
        exit 1
        ;;
esac

echo ""
echo "✓ Done!"
