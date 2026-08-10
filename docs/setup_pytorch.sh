#!/bin/bash
# Setup script for PyTorch demo
# Uses uv for fast Python package management

set -e

# Colors
COLOR_RESET='\033[0m'
COLOR_GREEN='\033[32m'
COLOR_BLUE='\033[34m'
COLOR_YELLOW='\033[33m'
COLOR_RED='\033[31m'

function write_success() {
    echo -e "${COLOR_GREEN}√${COLOR_RESET} $1"
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

function test_uv() {
    if ! command -v uv &> /dev/null; then
        write_error "uv is not installed!"
        write_info "Install from: https://astral.sh/uv/"
        write_info "Or run: curl -LsSf https://astral.sh/uv/install.sh | sh"
        return 1
    fi
    write_success "uv found: $(uv --version)"
    return 0
}

function install_dependencies() {
    write_info "Installing PyTorch dependencies with uv..."
    
    if ! test_uv; then
        exit 1
    fi
    
    # Check if requirements.txt exists
    if [ ! -f requirements.txt ]; then
        write_error "requirements.txt not found!"
        exit 1
    fi
    
    write_info "Installing from requirements.txt..."
    uv pip install -r requirements.txt
    
    write_success "Dependencies installed successfully!"
}

function run_demo() {
    write_info "Running PyTorch integration demo..."
    
    # Just use python directly - the demo has mock support
    python3 demo_pytorch.py
}

# Main script
echo ""
echo -e "${COLOR_BLUE}═══════════════════════════════════════════════════════${COLOR_RESET}"
echo -e "${COLOR_BLUE}  IronVault - PyTorch Demo Setup (uv)${COLOR_RESET}"
echo -e "${COLOR_BLUE}═══════════════════════════════════════════════════════${COLOR_RESET}"
echo ""

# Parse arguments
INSTALL=false
RUN=false
HELP=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --install)
            INSTALL=true
            shift
            ;;
        --run)
            RUN=true
            shift
            ;;
        --help|-h)
            HELP=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

if [ "$HELP" = true ]; then
    echo "Usage: ./setup_pytorch.sh [options]"
    echo ""
    echo "Options:"
    echo "  --install    Install PyTorch dependencies using uv"
    echo "  --run        Run the PyTorch demo"
    echo "  --help, -h   Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./setup_pytorch.sh --install        # Install dependencies"
    echo "  ./setup_pytorch.sh --run            # Run demo"
    echo "  ./setup_pytorch.sh --install --run  # Install and run"
    echo ""
    exit 0
fi

if [ "$INSTALL" = true ]; then
    install_dependencies
fi

if [ "$RUN" = true ]; then
    run_demo
fi

if [ "$INSTALL" = false ] && [ "$RUN" = false ]; then
    echo "No action specified. Use --help for usage information."
    echo ""
    echo "Quick start:"
    echo "  1. Install dependencies: ./setup_pytorch.sh --install"
    echo "  2. Run demo: ./setup_pytorch.sh --run"
    echo ""
fi
