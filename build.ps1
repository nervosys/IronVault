#!/usr/bin/env pwsh
# Build script for IronVault on Windows

param(
    [Parameter(Position = 0)]
    [ValidateSet('build', 'test', 'clean', 'install', 'format', 'lint', 'security', 'doc', 'release', 'all')]
    [string]$Command = 'build',
    
    [switch]$Release,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

function Write-Header {
    param([string]$Message)
    Write-Host "`n=== $Message ===" -ForegroundColor Cyan
}

function Build {
    Write-Header "Building IronVault"
    if ($Release) {
        cargo build --release
    }
    else {
        cargo build
    }
}

function Test {
    Write-Header "Running Tests"
    if ($Verbose) {
        cargo test --all-features -- --nocapture
    }
    else {
        cargo test --all-features
    }
}

function Clean {
    Write-Header "Cleaning Build Artifacts"
    cargo clean
    if (Test-Path "target") {
        Remove-Item -Recurse -Force target
    }
}

function Install {
    Write-Header "Installing IronVault"
    cargo install --path .
}

function Format {
    Write-Header "Formatting Code"
    cargo fmt
}

function Lint {
    Write-Header "Running Linters"
    cargo fmt -- --check
    cargo clippy -- -D warnings
}

function Security {
    Write-Header "Running Security Checks"
    
    Write-Host "Installing cargo-audit..." -ForegroundColor Yellow
    cargo install cargo-audit --quiet
    
    Write-Host "Running cargo audit..." -ForegroundColor Yellow
    cargo audit
    
    Write-Host "Installing cargo-deny..." -ForegroundColor Yellow
    cargo install cargo-deny --quiet
    
    Write-Host "Running cargo deny..." -ForegroundColor Yellow
    cargo deny check
}

function Documentation {
    Write-Header "Generating Documentation"
    cargo doc --no-deps --open
}

function Release-Build {
    Write-Header "Building Release"
    Format
    Lint
    Security
    Test
    cargo build --release --all-features
    
    Write-Host "`n✓ Release build complete!" -ForegroundColor Green
    Write-Host "Binary location: target\release\iv.exe" -ForegroundColor Green
}

function All {
    Format
    Lint
    Test
    Build
}

# Execute command
switch ($Command) {
    'build' { Build }
    'test' { Test }
    'clean' { Clean }
    'install' { Install }
    'format' { Format }
    'lint' { Lint }
    'security' { Security }
    'doc' { Documentation }
    'release' { Release-Build }
    'all' { All }
}

Write-Host "`n✓ Done!" -ForegroundColor Green
