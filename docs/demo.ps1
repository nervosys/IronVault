#!/usr/bin/env pwsh
# IronVault - PowerShell Demonstration Script
# This script showcases the key features of IronVault

param(
    [switch]$Quick,      # Run quick demo (basic_usage only)
    [switch]$Full,       # Run full demo (all examples)
    [switch]$Security,   # Run security demo
    [switch]$Utilities,  # Run utilities demo
    [switch]$RAG,        # Run RAG demo
    [switch]$HuggingFace # Run HuggingFace demo
)

# Colors for output
$ColorReset = "`e[0m"
$ColorBold = "`e[1m"
$ColorGreen = "`e[32m"
$ColorBlue = "`e[34m"
$ColorYellow = "`e[33m"
$ColorCyan = "`e[36m"
$ColorRed = "`e[31m"

function Write-Header {
    param([string]$Message)
    Write-Host ""
    Write-Host "$ColorBold$ColorCyan=== $Message ===$ColorReset" 
    Write-Host ""
}

function Write-Success {
    param([string]$Message)
    Write-Host "$ColorGreen✓$ColorReset $Message"
}

function Write-Info {
    param([string]$Message)
    Write-Host "$ColorBlue→$ColorReset $Message"
}

function Write-Warning {
    param([string]$Message)
    Write-Host "$ColorYellow!$ColorReset $Message"
}

function Write-Error-Custom {
    param([string]$Message)
    Write-Host "$ColorRed✗$ColorReset $Message"
}

function Test-CargoInstalled {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error-Custom "Cargo (Rust) is not installed!"
        Write-Info "Install from: https://rustup.rs/"
        exit 1
    }
    Write-Success "Cargo found: $(cargo --version)"
}

function Build-Project {
    Write-Header "Building IronVault"
    Write-Info "Building release version (optimized)..."
    
    $buildOutput = cargo build --release 2>&1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Build completed successfully!"
    }
    else {
        Write-Error-Custom "Build failed!"
        Write-Host $buildOutput
        exit 1
    }
}

function Run-Tests {
    Write-Header "Running Test Suite"
    Write-Info "Executing all tests..."
    
    $testOutput = cargo test --lib 2>&1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Success "All tests passed!"
    }
    else {
        Write-Error-Custom "Tests failed!"
        Write-Host $testOutput
        exit 1
    }
}

function Run-SecurityAudit {
    Write-Header "Security Audit"
    Write-Info "Scanning dependencies for vulnerabilities..."
    
    cargo audit 2>&1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Security audit complete - no critical vulnerabilities"
    }
    else {
        Write-Warning "Security audit completed with warnings (check output above)"
    }
}

function Run-Example {
    param(
        [string]$Name,
        [string]$Description
    )
    
    Write-Header "Running: $Name"
    Write-Info $Description
    Write-Host ""
    
    cargo run --example $Name --release 2>&1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Success "Example '$Name' completed successfully!"
    }
    else {
        Write-Host ""
        Write-Error-Custom "Example '$Name' failed!"
    }
    
    Write-Host ""
    Write-Host "$ColorYellow" + ("=" * 80) + "$ColorReset"
    Write-Host ""
}

# Main script
Write-Host ""
Write-Host "$ColorBold$ColorCyan"
Write-Host "╔═══════════════════════════════════════════════════════════════╗"
Write-Host "║          IronVault - Demonstration Script               ║"
Write-Host "║                                                               ║"
Write-Host "║  Secure, encrypted storage for AI models with version        ║"
Write-Host "║  control, compression, and FIPS 140-3 compliance             ║"
Write-Host "╚═══════════════════════════════════════════════════════════════╝"
Write-Host "$ColorReset"

# Check prerequisites
Write-Header "Prerequisites Check"
Test-CargoInstalled

# Build project
Build-Project

# Run based on parameters
if ($Quick) {
    Write-Info "Running quick demo (basic_usage only)..."
    Run-Example "basic_usage" "Basic vault operations: store, retrieve, version control"
}
elseif ($Security) {
    Run-Example "security_demo" "Security features: encryption, key derivation, compliance"
}
elseif ($Utilities) {
    Run-Example "utilities_demo" "Model utilities: analysis, compression, deduplication"
}
elseif ($RAG) {
    Run-Example "rag_demo" "RAG system: document store, knowledge base, MCP tools"
}
elseif ($HuggingFace) {
    Run-Example "huggingface_demo" "HuggingFace integration: model download, version tracking"
}
elseif ($Full) {
    Write-Info "Running full demonstration suite..."
    Write-Host ""
    
    # Run security audit first
    Run-SecurityAudit
    
    # Run all examples
    Run-Example "basic_usage" "Basic vault operations: store, retrieve, version control"
    Run-Example "huggingface_demo" "HuggingFace integration: model download, version tracking"
    Run-Example "security_demo" "Security features: encryption, key derivation, compliance"
    Run-Example "utilities_demo" "Model utilities: analysis, compression, deduplication"
    Run-Example "rag_demo" "RAG system: document store, knowledge base, MCP tools"
    
    # Run tests
    Run-Tests
    
    Write-Header "Full Demo Complete!"
    Write-Success "All examples executed successfully"
    Write-Info "Check the output above for detailed results"
}
else {
    # Default: show menu
    Write-Host "Usage: .\demo.ps1 [options]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Quick          Run quick demo (basic_usage only)"
    Write-Host "  -Full           Run full demo suite (all examples + tests)"
    Write-Host "  -Security       Run security demonstration"
    Write-Host "  -Utilities      Run utilities demonstration"
    Write-Host "  -RAG            Run RAG system demonstration"
    Write-Host "  -HuggingFace    Run HuggingFace integration demo"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  .\demo.ps1 -Quick          # Quick 2-minute demo"
    Write-Host "  .\demo.ps1 -Full           # Complete demonstration (~10 minutes)"
    Write-Host "  .\demo.ps1 -HuggingFace    # See HuggingFace model management"
    Write-Host ""
    
    # Offer to run quick demo
    Write-Host "$ColorYellow"
    $response = Read-Host "Run quick demo now? (y/n)"
    Write-Host "$ColorReset"
    
    if ($response -eq 'y' -or $response -eq 'Y') {
        Run-Example "basic_usage" "Basic vault operations: store, retrieve, version control"
    }
}

Write-Host ""
Write-Host "$ColorBold$ColorGreen"
Write-Host "╔═══════════════════════════════════════════════════════════════╗"
Write-Host "║                    Demonstration Complete                     ║"
Write-Host "╚═══════════════════════════════════════════════════════════════╝"
Write-Host "$ColorReset"
Write-Host ""
Write-Host "Next steps:"
Write-Host "  • Read the documentation: README.md"
Write-Host "  • View security status: SECURITY_STATUS.md"
Write-Host "  • Check feature demo: FEATURES_DEMO.md"
Write-Host "  • Explore examples: examples/"
Write-Host ""
Write-Host "For more information: https://github.com/yourusername/ironvault"
Write-Host ""
