#!/usr/bin/env pwsh
# Setup script for PyTorch demo
# Uses uv for fast Python package management

param(
    [switch]$Install,    # Install dependencies
    [switch]$Run,        # Run the demo
    [switch]$Help        # Show help
)

$ColorReset = "`e[0m"
$ColorGreen = "`e[32m"
$ColorBlue = "`e[34m"
$ColorYellow = "`e[33m"
$ColorRed = "`e[31m"

function Write-Success {
    param([string]$Message)
    Write-Host "$ColorGreen√$ColorReset $Message"
}

function Write-Info {
    param([string]$Message)
    Write-Host "$ColorBlue→$ColorReset $Message"
}

function Write-Warning {
    param([string]$Message)
    Write-Host "$ColorYellow!$ColorReset $Message"
}

function Write-ErrorMsg {
    param([string]$Message)
    Write-Host "$ColorRed✗$ColorReset $Message"
}

function Test-UV {
    if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
        Write-ErrorMsg "uv is not installed!"
        Write-Info "Install from: https://astral.sh/uv/"
        Write-Info "Or run: powershell -c `"irm https://astral.sh/uv/install.ps1 | iex`""
        return $false
    }
    Write-Success "uv found: $(uv --version)"
    return $true
}

function Install-Dependencies {
    Write-Info "Installing PyTorch dependencies with uv..."
    
    if (-not (Test-UV)) {
        exit 1
    }
    
    # Check if requirements.txt exists
    if (-not (Test-Path requirements.txt)) {
        Write-ErrorMsg "requirements.txt not found!"
        exit 1
    }
    
    Write-Info "Installing from requirements.txt..."
    uv pip install -r requirements.txt
    
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Dependencies installed successfully!"
    }
    else {
        Write-ErrorMsg "Failed to install dependencies"
        exit 1
    }
}

function Start-Demo {
    Write-Info "Running PyTorch integration demo..."
    
    # Just use python directly - the demo has mock support
    python demo_pytorch.py
}

# Main script
Write-Host ""
Write-Host "$ColorBlue═══════════════════════════════════════════════════════$ColorReset"
Write-Host "$ColorBlue  IronVault - PyTorch Demo Setup (uv)$ColorReset"
Write-Host "$ColorBlue═══════════════════════════════════════════════════════$ColorReset"
Write-Host ""

if ($Help) {
    Write-Host "Usage: .\setup_pytorch.ps1 [options]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Install    Install PyTorch dependencies using uv"
    Write-Host "  -Run        Run the PyTorch demo"
    Write-Host "  -Help       Show this help message"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  .\setup_pytorch.ps1 -Install    # Install dependencies"
    Write-Host "  .\setup_pytorch.ps1 -Run        # Run demo"
    Write-Host "  .\setup_pytorch.ps1 -Install -Run  # Install and run"
    Write-Host ""
    exit 0
}

if ($Install) {
    Install-Dependencies
}

if ($Run) {
    Start-Demo
}

if (-not $Install -and -not $Run) {
    Write-Host "No action specified. Use -Help for usage information."
    Write-Host ""
    Write-Host "Quick start:"
    Write-Host "  1. Install dependencies: .\setup_pytorch.ps1 -Install"
    Write-Host "  2. Run demo: .\setup_pytorch.ps1 -Run"
    Write-Host ""
}
