#!/usr/bin/env pwsh
# Quick test and validation script for ironvault

Write-Host "=== ironvault Quick Validation ===" -ForegroundColor Cyan
Write-Host ""

# 1. Format check
Write-Host "1. Checking code formatting..." -ForegroundColor Yellow
$formatResult = cargo fmt -- --check 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "   ✓ Code formatting OK" -ForegroundColor Green
}
else {
    Write-Host "   ✗ Code needs formatting (run 'cargo fmt')" -ForegroundColor Red
}

# 2. Clippy check
Write-Host "`n2. Running Clippy linter..." -ForegroundColor Yellow
$clippyResult = cargo clippy --quiet 2>&1 | Select-String -Pattern "warning|error" | Select-Object -First 10
if ($clippyResult) {
    Write-Host "   Warnings/Errors found:" -ForegroundColor Yellow
    $clippyResult | ForEach-Object { Write-Host "   $_" }
}
else {
    Write-Host "   ✓ No clippy warnings" -ForegroundColor Green
}

# 3. Build check
Write-Host "`n3. Building project..." -ForegroundColor Yellow
cargo build --quiet 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "   ✓ Build successful" -ForegroundColor Green
}
else {
    Write-Host "   ✗ Build failed" -ForegroundColor Red
    exit 1
}

# 4. Run tests
Write-Host "`n4. Running tests..." -ForegroundColor Yellow
$testOutput = cargo test --quiet 2>&1
$testPassed = $testOutput | Select-String -Pattern "test result: ok"
if ($testPassed) {
    Write-Host "   ✓ Tests passed" -ForegroundColor Green
    $testSummary = $testOutput | Select-String -Pattern "(\d+) passed"
    if ($testSummary) {
        Write-Host "   $testSummary" -ForegroundColor Cyan
    }
}
else {
    Write-Host "   ✗ Some tests failed" -ForegroundColor Red
    $testOutput | Select-String -Pattern "FAILED|error" | ForEach-Object { Write-Host "   $_" }
}

# 5. Check documentation
Write-Host "`n5. Checking documentation..." -ForegroundColor Yellow
cargo doc --no-deps --quiet 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "   ✓ Documentation builds" -ForegroundColor Green
}
else {
    Write-Host "   ✗ Documentation has errors" -ForegroundColor Red
}

# Summary
Write-Host "`n=== Validation Complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor White
Write-Host "  - Run examples: cargo run --example basic_usage" -ForegroundColor Gray
Write-Host "  - Build release: cargo build --release" -ForegroundColor Gray
Write-Host "  - Install: cargo install --path ." -ForegroundColor Gray
Write-Host ""
