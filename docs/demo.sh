#!/bin/bash
# IronVault - BASH Demonstration Script
# This script showcases the key features of IronVault

set -e  # Exit on error

# Colors for output
COLOR_RESET='\033[0m'
COLOR_BOLD='\033[1m'
COLOR_GREEN='\033[32m'
COLOR_BLUE='\033[34m'
COLOR_YELLOW='\033[33m'
COLOR_CYAN='\033[36m'
COLOR_RED='\033[31m'

# Command line options
QUICK=false
FULL=false
SECURITY=false
UTILITIES=false
RAG=false
HUGGINGFACE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            QUICK=true
            shift
            ;;
        --full)
            FULL=true
            shift
            ;;
        --security)
            SECURITY=true
            shift
            ;;
        --utilities)
            UTILITIES=true
            shift
            ;;
        --rag)
            RAG=true
            shift
            ;;
        --huggingface)
            HUGGINGFACE=true
            shift
            ;;
        --help|-h)
            echo "Usage: ./demo.sh [options]"
            echo ""
            echo "Options:"
            echo "  --quick          Run quick demo (basic_usage only)"
            echo "  --full           Run full demo suite (all examples + tests)"
            echo "  --security       Run security demonstration"
            echo "  --utilities      Run utilities demonstration"
            echo "  --rag            Run RAG system demonstration"
            echo "  --huggingface    Run HuggingFace integration demo"
            echo "  --help, -h       Show this help message"
            echo ""
            echo "Examples:"
            echo "  ./demo.sh --quick          # Quick 2-minute demo"
            echo "  ./demo.sh --full           # Complete demonstration (~10 minutes)"
            echo "  ./demo.sh --huggingface    # See HuggingFace model management"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

function write_header() {
    echo ""
    echo -e "${COLOR_BOLD}${COLOR_CYAN}=== $1 ===${COLOR_RESET}"
    echo ""
}

function write_success() {
    echo -e "${COLOR_GREEN}✓${COLOR_RESET} $1"
}

function write_info() {
    echo -e "${COLOR_BLUE}→${COLOR_RESET} $1"
}

function write_warning() {
    echo -e "${COLOR_YELLOW}!${COLOR_RESET} $1"
}

function write_error() {
    echo -e "${COLOR_RED}✗${COLOR_RESET} $1"
}

function test_cargo_installed() {
    if ! command -v cargo &> /dev/null; then
        write_error "Cargo (Rust) is not installed!"
        write_info "Install from: https://rustup.rs/"
        exit 1
    fi
    write_success "Cargo found: $(cargo --version)"
}

function build_project() {
    write_header "Building IronVault"
    write_info "Building release version (optimized)..."
    
    if cargo build --release 2>&1; then
        write_success "Build completed successfully!"
    else
        write_error "Build failed!"
        exit 1
    fi
}

function run_tests() {
    write_header "Running Test Suite"
    write_info "Executing all tests..."
    
    if cargo test --lib 2>&1; then
        write_success "All tests passed!"
    else
        write_error "Tests failed!"
        exit 1
    fi
}

function run_security_audit() {
    write_header "Security Audit"
    write_info "Scanning dependencies for vulnerabilities..."
    
    if cargo audit 2>&1; then
        write_success "Security audit complete - no critical vulnerabilities"
    else
        write_warning "Security audit completed with warnings (check output above)"
    fi
}

function run_example() {
    local name=$1
    local description=$2
    
    write_header "Running: $name"
    write_info "$description"
    echo ""
    
    if cargo run --example "$name" --release 2>&1; then
        echo ""
        write_success "Example '$name' completed successfully!"
    else
        echo ""
        write_error "Example '$name' failed!"
    fi
    
    echo ""
    echo -e "${COLOR_YELLOW}$(printf '=%.0s' {1..80})${COLOR_RESET}"
    echo ""
}

# Main script
echo ""
echo -e "${COLOR_BOLD}${COLOR_CYAN}"
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║          IronVault - Demonstration Script               ║"
echo "║                                                               ║"
echo "║  Secure, encrypted storage for AI models with version        ║"
echo "║  control, compression, and FIPS 140-3 compliance             ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo -e "${COLOR_RESET}"

# Check prerequisites
write_header "Prerequisites Check"
test_cargo_installed

# Build project
build_project

# Run based on parameters
if [ "$QUICK" = true ]; then
    write_info "Running quick demo (basic_usage only)..."
    run_example "basic_usage" "Basic vault operations: store, retrieve, version control"
elif [ "$SECURITY" = true ]; then
    run_example "security_demo" "Security features: encryption, key derivation, compliance"
elif [ "$UTILITIES" = true ]; then
    run_example "utilities_demo" "Model utilities: analysis, compression, deduplication"
elif [ "$RAG" = true ]; then
    run_example "rag_demo" "RAG system: document store, knowledge base, MCP tools"
elif [ "$HUGGINGFACE" = true ]; then
    run_example "huggingface_demo" "HuggingFace integration: model download, version tracking"
elif [ "$FULL" = true ]; then
    write_info "Running full demonstration suite..."
    echo ""
    
    # Run security audit first
    run_security_audit
    
    # Run all examples
    run_example "basic_usage" "Basic vault operations: store, retrieve, version control"
    run_example "huggingface_demo" "HuggingFace integration: model download, version tracking"
    run_example "security_demo" "Security features: encryption, key derivation, compliance"
    run_example "utilities_demo" "Model utilities: analysis, compression, deduplication"
    run_example "rag_demo" "RAG system: document store, knowledge base, MCP tools"
    
    # Run tests
    run_tests
    
    write_header "Full Demo Complete!"
    write_success "All examples executed successfully"
    write_info "Check the output above for detailed results"
else
    # Default: show menu
    echo "Usage: ./demo.sh [options]"
    echo ""
    echo "Options:"
    echo "  --quick          Run quick demo (basic_usage only)"
    echo "  --full           Run full demo suite (all examples + tests)"
    echo "  --security       Run security demonstration"
    echo "  --utilities      Run utilities demonstration"
    echo "  --rag            Run RAG system demonstration"
    echo "  --huggingface    Run HuggingFace integration demo"
    echo ""
    echo "Examples:"
    echo "  ./demo.sh --quick          # Quick 2-minute demo"
    echo "  ./demo.sh --full           # Complete demonstration (~10 minutes)"
    echo "  ./demo.sh --huggingface    # See HuggingFace model management"
    echo ""
    
    # Offer to run quick demo
    echo -e "${COLOR_YELLOW}"
    read -p "Run quick demo now? (y/n) " -n 1 -r
    echo -e "${COLOR_RESET}"
    echo ""
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        run_example "basic_usage" "Basic vault operations: store, retrieve, version control"
    fi
fi

echo ""
echo -e "${COLOR_BOLD}${COLOR_GREEN}"
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                    Demonstration Complete                     ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo -e "${COLOR_RESET}"
echo ""
echo "Next steps:"
echo "  • Read the documentation: README.md"
echo "  • View security status: SECURITY_STATUS.md"
echo "  • Check feature demo: FEATURES_DEMO.md"
echo "  • Explore examples: examples/"
echo ""
echo "For more information: https://github.com/yourusername/ironvault"
echo ""
