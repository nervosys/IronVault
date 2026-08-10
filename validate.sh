#!/usr/bin/env bash
# Quick test and validation script for ironvault

set -e

echo "=== ironvault Quick Validation ==="
echo ""

# 1. Format check
echo "1. Checking code formatting..."
if cargo fmt -- --check >/dev/null 2>&1; then
    echo "   ✓ Code formatting OK"
else
    echo "   ✗ Code needs formatting (run 'cargo fmt')"
fi

# 2. Clippy check  
echo ""
echo "2. Running Clippy linter..."
if cargo clippy --quiet 2>&1 | grep -E "warning|error" | head -10; then
    echo "   Warnings/Errors found (see above)"
else
    echo "   ✓ No clippy warnings"
fi

# 3. Build check
echo ""
echo "3. Building project..."
if cargo build --quiet 2>&1; then
    echo "   ✓ Build successful"
else
    echo "   ✗ Build failed"
    exit 1
fi

# 4. Run tests
echo ""
echo "4. Running tests..."
if cargo test --quiet 2>&1 | grep "test result: ok"; then
    echo "   ✓ Tests passed"
else
    echo "   ✗ Some tests failed"
fi

# 5. Check documentation
echo ""
echo "5. Checking documentation..."
if cargo doc --no-deps --quiet 2>&1; then
    echo "   ✓ Documentation builds"
else
    echo "   ✗ Documentation has errors"
fi

# Summary
echo ""
echo "=== Validation Complete ==="
echo ""
echo "Next steps:"
echo "  - Run examples: cargo run --example basic_usage"
echo "  - Build release: cargo build --release"
echo "  - Install: cargo install --path ."
echo ""
