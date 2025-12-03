# EventSleuth Test Runner Script
# This script runs all tests (frontend and backend) and generates reports

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "   EventSleuth Test Suite Runner     " -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

$ErrorActionPreference = "Continue"
$frontendPassed = $false
$backendPassed = $false
$startTime = Get-Date

# Function to print section headers
function Write-Section {
    param([string]$title)
    Write-Host ""
    Write-Host "======================================" -ForegroundColor Yellow
    Write-Host " $title" -ForegroundColor Yellow
    Write-Host "======================================" -ForegroundColor Yellow
    Write-Host ""
}

# Function to print success message
function Write-Success {
    param([string]$message)
    Write-Host "✓ $message" -ForegroundColor Green
}

# Function to print error message
function Write-Error-Message {
    param([string]$message)
    Write-Host "✗ $message" -ForegroundColor Red
}

# Function to print info message
function Write-Info {
    param([string]$message)
    Write-Host "ℹ $message" -ForegroundColor Blue
}

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "⚠️  WARNING: Not running as Administrator" -ForegroundColor Yellow
    Write-Host "   Some backend tests may fail without admin privileges" -ForegroundColor Yellow
    Write-Host "   To run as admin: Right-click PowerShell -> 'Run as Administrator'" -ForegroundColor Yellow
    Write-Host ""

    $response = Read-Host "Continue anyway? (y/n)"
    if ($response -ne 'y' -and $response -ne 'Y') {
        Write-Host "Exiting..." -ForegroundColor Yellow
        exit 0
    }
}

# Navigate to project root
$scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptPath

Write-Info "Project directory: $scriptPath"
Write-Info "Running as Administrator: $isAdmin"
Write-Host ""

# ===========================================
# FRONTEND TESTS
# ===========================================
Write-Section "Running Frontend Tests (Vitest)"

try {
    Write-Info "Installing/checking dependencies..."
    npm install --silent

    Write-Info "Running frontend test suite..."
    Write-Host ""

    $frontendResult = npm test -- --run --reporter=verbose 2>&1

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Frontend tests passed!"
        $frontendPassed = $true
    } else {
        Write-Error-Message "Frontend tests failed!"
        Write-Host $frontendResult
    }
} catch {
    Write-Error-Message "Error running frontend tests: $_"
}

# ===========================================
# FRONTEND COVERAGE
# ===========================================
Write-Section "Generating Frontend Coverage Report"

try {
    Write-Info "Running tests with coverage..."
    npm run test:coverage -- --run 2>&1 | Out-Null

    if (Test-Path "coverage/index.html") {
        Write-Success "Coverage report generated: coverage/index.html"

        # Parse coverage summary if available
        if (Test-Path "coverage/coverage-summary.json") {
            $coverage = Get-Content "coverage/coverage-summary.json" | ConvertFrom-Json
            $total = $coverage.total

            Write-Host ""
            Write-Host "Coverage Summary:" -ForegroundColor Cyan
            Write-Host "  Lines:      $($total.lines.pct)%" -ForegroundColor $(if ($total.lines.pct -ge 80) { "Green" } else { "Yellow" })
            Write-Host "  Statements: $($total.statements.pct)%" -ForegroundColor $(if ($total.statements.pct -ge 80) { "Green" } else { "Yellow" })
            Write-Host "  Functions:  $($total.functions.pct)%" -ForegroundColor $(if ($total.functions.pct -ge 80) { "Green" } else { "Yellow" })
            Write-Host "  Branches:   $($total.branches.pct)%" -ForegroundColor $(if ($total.branches.pct -ge 80) { "Green" } else { "Yellow" })
        }
    } else {
        Write-Error-Message "Coverage report not generated"
    }
} catch {
    Write-Error-Message "Error generating coverage: $_"
}

# ===========================================
# BACKEND TESTS
# ===========================================
Write-Section "Running Backend Tests (Rust/Cargo)"

try {
    Set-Location "src-tauri"

    Write-Info "Checking Rust toolchain..."
    $rustVersion = cargo --version
    Write-Info "Using: $rustVersion"

    Write-Info "Running backend test suite..."
    Write-Host ""

    $backendResult = cargo test --color=always 2>&1

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Backend tests passed!"
        $backendPassed = $true
    } else {
        Write-Error-Message "Backend tests failed!"
        Write-Host $backendResult
    }

    Set-Location ".."
} catch {
    Write-Error-Message "Error running backend tests: $_"
    Set-Location ".."
}

# ===========================================
# BACKEND TESTS (DETAILED)
# ===========================================
Write-Section "Running Backend Tests (Verbose Output)"

try {
    Set-Location "src-tauri"

    Write-Info "Running tests with detailed output..."
    Write-Host ""

    cargo test -- --nocapture --test-threads=1 2>&1

    Set-Location ".."
} catch {
    Write-Error-Message "Error running detailed backend tests: $_"
    Set-Location ".."
}

# ===========================================
# SUMMARY
# ===========================================
Write-Section "Test Summary"

$endTime = Get-Date
$duration = $endTime - $startTime

Write-Host "Execution Time: $($duration.TotalSeconds) seconds" -ForegroundColor Cyan
Write-Host ""

if ($frontendPassed) {
    Write-Success "Frontend Tests: PASSED"
} else {
    Write-Error-Message "Frontend Tests: FAILED"
}

if ($backendPassed) {
    Write-Success "Backend Tests: PASSED"
} else {
    Write-Error-Message "Backend Tests: FAILED"
}

Write-Host ""

if ($frontendPassed -and $backendPassed) {
    Write-Host "======================================" -ForegroundColor Green
    Write-Host "   ALL TESTS PASSED! ✓               " -ForegroundColor Green
    Write-Host "======================================" -ForegroundColor Green

    # Open coverage report
    $openCoverage = Read-Host "Open coverage report in browser? (y/n)"
    if ($openCoverage -eq 'y' -or $openCoverage -eq 'Y') {
        if (Test-Path "coverage/index.html") {
            Start-Process "coverage/index.html"
        }
    }

    exit 0
} else {
    Write-Host "======================================" -ForegroundColor Red
    Write-Host "   SOME TESTS FAILED ✗               " -ForegroundColor Red
    Write-Host "======================================" -ForegroundColor Red

    if (-not $isAdmin) {
        Write-Host ""
        Write-Host "Note: Some tests may require Administrator privileges" -ForegroundColor Yellow
    }

    exit 1
}
