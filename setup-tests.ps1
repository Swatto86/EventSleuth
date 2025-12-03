# EventSleuth Test Setup Script
# This script installs all necessary dependencies for running tests

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  EventSleuth Test Setup             " -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

$ErrorActionPreference = "Stop"

# Navigate to project root
$scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptPath

Write-Host "Installing Node.js dependencies..." -ForegroundColor Yellow
npm install

Write-Host ""
Write-Host "Installing additional test dependencies..." -ForegroundColor Yellow
npm install -D @testing-library/user-event@^14.5.1 @vitest/coverage-v8@^1.0.0 jsdom@^23.0.0

Write-Host ""
Write-Host "Verifying Rust toolchain..." -ForegroundColor Yellow
$rustVersion = cargo --version
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Rust installed: $rustVersion" -ForegroundColor Green
} else {
    Write-Host "✗ Rust not found. Please install from https://rustup.rs/" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "======================================" -ForegroundColor Green
Write-Host "  Setup Complete!                    " -ForegroundColor Green
Write-Host "======================================" -ForegroundColor Green
Write-Host ""
Write-Host "You can now run tests with:" -ForegroundColor Cyan
Write-Host "  npm test              # Frontend tests" -ForegroundColor White
Write-Host "  npm run test:backend  # Backend tests" -ForegroundColor White
Write-Host "  .\run-all-tests.ps1   # All tests" -ForegroundColor White
Write-Host ""
