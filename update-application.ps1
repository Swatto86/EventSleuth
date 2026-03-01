# EventSleuth Release Script
# =========================
# Automates the complete release pipeline per Rule 18 of the Engineering Contract.
#
# Usage:
#   Interactive:      .\update-application.ps1
#   Parameterised:    .\update-application.ps1 -Version "1.1.0" -Notes "Added feature X"
#   Force (skip dup): .\update-application.ps1 -Version "1.0.2" -Notes "Hotfix" -Force
#
# Steps:
#   1. Version validation (semver, duplicate check, comparison)
#   2. Manifest updates (Cargo.toml)
#   3. Pre-release validation (build release + run full test suite)
#   4. Installer build (NSIS, if makensis available)
#   5. Release notes collection
#   6. Git operations (commit, annotated tag, push)
#   7. Tag cleanup (delete all previous tags and GitHub releases)
#   8. CI/CD trigger (tag push triggers GitHub Actions)
#
# On failure at any step, the script rolls back version changes and aborts.

param(
    [string]$Version,
    [string]$Notes,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

# ── Helpers ─────────────────────────────────────────────────────────────

function Write-Success { param($msg) Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Info    { param($msg) Write-Host "  [..] $msg" -ForegroundColor Cyan }
function Write-Warn    { param($msg) Write-Host "  [!!] $msg" -ForegroundColor Yellow }
function Write-Fail    { param($msg) Write-Host "  [FAIL] $msg" -ForegroundColor Red }
function Write-Step    { param($num, $label) Write-Host "`n=== Step $num: $label ===" -ForegroundColor Magenta }

function Rollback-Version {
    param([string]$OriginalContent, [string]$FilePath)
    Write-Warn "Rolling back version change in $FilePath..."
    Set-Content $FilePath $OriginalContent -NoNewline
    # Unstage the file if it was git-added
    git checkout -- $FilePath 2>&1 | Out-Null
    Write-Warn "Rollback complete."
}

# ── Setup ───────────────────────────────────────────────────────────────

$projectRoot = $PSScriptRoot
Set-Location $projectRoot

Write-Host ""
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   EventSleuth Release Pipeline" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta

$cargoToml = Join-Path $projectRoot "Cargo.toml"
if (-not (Test-Path $cargoToml)) {
    Write-Fail "Cargo.toml not found at $cargoToml"
    exit 1
}

# Read current version from Cargo.toml
$cargoOriginal = Get-Content $cargoToml -Raw
$currentMatch = [regex]::Match($cargoOriginal, '(?m)^version = "([^"]+)"')
if (-not $currentMatch.Success) {
    Write-Fail "Could not parse current version from Cargo.toml"
    exit 1
}
$currentVersion = $currentMatch.Groups[1].Value
Write-Info "Current version: $currentVersion"

# ── Step 1: Version validation ──────────────────────────────────────────

Write-Step 1 "Version Validation"

if (-not $Version) {
    $Version = Read-Host "Enter the new version (semver, e.g. 1.1.0)"
}

if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    Write-Fail "Invalid version format '$Version'. Must be semver: MAJOR.MINOR.PATCH"
    exit 1
}

# Parse versions for comparison
function Parse-SemVer([string]$v) {
    $parts = $v -split '\.'
    return @{ Major = [int]$parts[0]; Minor = [int]$parts[1]; Patch = [int]$parts[2] }
}

$newVer = Parse-SemVer $Version
$curVer = Parse-SemVer $currentVersion

# Check for duplicate
if ($Version -eq $currentVersion -and -not $Force) {
    Write-Fail "Version $Version is the same as current. Use -Force to override."
    exit 1
}

# Check version is not lower (unless forced)
$newNum = $newVer.Major * 10000 + $newVer.Minor * 100 + $newVer.Patch
$curNum = $curVer.Major * 10000 + $curVer.Minor * 100 + $curVer.Patch
if ($newNum -lt $curNum -and -not $Force) {
    Write-Fail "Version $Version is lower than current $currentVersion. Use -Force to override."
    exit 1
}

Write-Success "Version $Version validated (current: $currentVersion)"

# ── Step 2: Manifest updates ────────────────────────────────────────────

Write-Step 2 "Manifest Updates"

$cargoContent = $cargoOriginal -replace '(?m)^version = "[^"]+"', "version = `"$Version`""
Set-Content $cargoToml $cargoContent -NoNewline
Write-Success "Updated Cargo.toml: $currentVersion -> $Version"

# ── Step 3: Pre-release validation (build + test) ───────────────────────

Write-Step 3 "Pre-release Validation"

Write-Info "Building release configuration..."
$buildResult = & cargo build --release 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Fail "Release build failed:"
    Write-Host ($buildResult | Out-String)
    Rollback-Version -OriginalContent $cargoOriginal -FilePath $cargoToml
    exit 1
}
Write-Success "Release build succeeded"

Write-Info "Running test suite..."
$testResult = & cargo test 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Fail "Tests failed:"
    Write-Host ($testResult | Out-String)
    Rollback-Version -OriginalContent $cargoOriginal -FilePath $cargoToml
    exit 1
}
Write-Success "All tests passed"

Write-Info "Running clippy..."
$clippyResult = & cargo clippy -- -D warnings 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Warn "Clippy warnings detected (non-blocking):"
    Write-Host ($clippyResult | Out-String)
} else {
    Write-Success "Clippy clean"
}

# ── Step 4: Installer build ─────────────────────────────────────────────

Write-Step 4 "Installer Build"

$nsisAvailable = $null -ne (Get-Command "makensis" -ErrorAction SilentlyContinue)
$nsiScript = Join-Path $projectRoot "installer\eventsleuth.nsi"

if ($nsisAvailable -and (Test-Path $nsiScript)) {
    Write-Info "Building NSIS installer..."
    $nsisResult = & makensis /DPRODUCT_VERSION=$Version "$nsiScript" 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Warn "Installer build failed (non-blocking):"
        Write-Host ($nsisResult | Out-String)
    } else {
        $setupExe = Join-Path $projectRoot "EventSleuth-$Version-Setup.exe"
        if (Test-Path $setupExe) {
            $size = [math]::Round((Get-Item $setupExe).Length / 1MB, 2)
            Write-Success "Installer built: EventSleuth-$Version-Setup.exe ($size MB)"
        } else {
            Write-Warn "Installer executable not found at expected path"
        }
    }
} else {
    if (-not $nsisAvailable) {
        Write-Warn "NSIS (makensis) not found on PATH - skipping installer build"
        Write-Warn "Install NSIS from https://nsis.sourceforge.io to build installers"
    }
    if (-not (Test-Path $nsiScript)) {
        Write-Warn "Installer script not found at $nsiScript"
    }
}

# ── Step 5: Release notes ───────────────────────────────────────────────

Write-Step 5 "Release Notes"

if (-not $Notes) {
    Write-Host "Enter release notes (what changed in this version)." -ForegroundColor Cyan
    Write-Host "These notes will appear on the GitHub Releases page." -ForegroundColor Yellow
    Write-Host "Type 'END' on a new line when done:" -ForegroundColor Gray
    Write-Host ""

    $noteLines = @()
    while ($true) {
        $line = Read-Host
        if ($line -eq 'END') { break }
        $noteLines += $line
    }
    $Notes = $noteLines -join "`n"
}

if (-not $Notes -or $Notes.Trim() -eq '') {
    $Notes = "Release v$Version"
}

Write-Host ""
Write-Info "Release notes:"
Write-Host "---"
Write-Host $Notes
Write-Host "---"
Write-Host ""

if (-not $Force) {
    $confirm = Read-Host "Proceed with release v$Version? (Y/n)"
    if ($confirm -eq 'n' -or $confirm -eq 'N') {
        Rollback-Version -OriginalContent $cargoOriginal -FilePath $cargoToml
        Write-Fail "Aborted by user."
        exit 1
    }
}

# ── Check for uncommitted changes ───────────────────────────────────────

$gitStatus = git status --porcelain
if ($gitStatus) {
    # Filter out the Cargo.toml change we just made
    $otherChanges = ($gitStatus -split "`n") | Where-Object { $_ -and $_ -notmatch 'Cargo\.toml' }
    if ($otherChanges) {
        Write-Warn "Uncommitted changes detected:"
        Write-Host ($otherChanges -join "`n")
        if (-not $Force) {
            $confirm = Read-Host "Continue with uncommitted changes? (Y/n)"
            if ($confirm -eq 'n' -or $confirm -eq 'N') {
                Rollback-Version -OriginalContent $cargoOriginal -FilePath $cargoToml
                Write-Fail "Aborted by user."
                exit 1
            }
        }
    }
}

# ── Step 6: Git operations ──────────────────────────────────────────────

Write-Step 6 "Git Operations"

Write-Info "Committing version bump..."
git add $cargoToml
git commit -m "chore: bump version to $Version"
if ($LASTEXITCODE -ne 0) {
    Write-Fail "Git commit failed"
    Rollback-Version -OriginalContent $cargoOriginal -FilePath $cargoToml
    exit 1
}
Write-Success "Committed version bump"

# ── Step 7: Tag cleanup ─────────────────────────────────────────────────

Write-Step 7 "Tag Cleanup"

$existingTags = git tag -l "v*"
if ($existingTags) {
    Write-Info "Found existing tags: $($existingTags -join ', ')"

    $ghAvailable = $null -ne (Get-Command gh -ErrorAction SilentlyContinue)

    if ($ghAvailable) {
        Write-Info "Deleting GitHub releases..."
        $ErrorActionPreference = "Continue"
        foreach ($tag in $existingTags) {
            $tag = $tag.Trim()
            if ($tag) {
                gh release delete $tag --yes 2>&1 | Out-Null
            }
        }
        $ErrorActionPreference = "Stop"
        Write-Success "Deleted GitHub releases"
    } else {
        Write-Warn "GitHub CLI (gh) not found - skipping release deletion"
    }

    Write-Info "Deleting local tags..."
    foreach ($tag in $existingTags) {
        $tag = $tag.Trim()
        if ($tag) { git tag -d $tag 2>&1 | Out-Null }
    }
    Write-Success "Deleted local tags"

    Write-Info "Deleting remote tags..."
    $ErrorActionPreference = "Continue"
    foreach ($tag in $existingTags) {
        $tag = $tag.Trim()
        if ($tag) { git push origin --delete $tag 2>&1 | Out-Null }
    }
    $ErrorActionPreference = "Stop"
    Write-Success "Deleted remote tags"
} else {
    Write-Info "No existing tags found"
}

# Create annotated tag
Write-Info "Creating annotated tag v$Version..."
$tempFile = [System.IO.Path]::GetTempFileName()
Set-Content $tempFile $Notes -NoNewline
git tag -a "v$Version" -F $tempFile
Remove-Item $tempFile
Write-Success "Created tag v$Version"

# ── Step 8: Push (triggers CI/CD) ───────────────────────────────────────

Write-Step 8 "Push to Origin (triggers CI)"

$ErrorActionPreference = "Continue"
git push origin HEAD 2>&1 | Out-Null
$pushCode = $LASTEXITCODE
git push origin "v$Version" 2>&1 | Out-Null
$tagPushCode = $LASTEXITCODE
$ErrorActionPreference = "Stop"

if ($pushCode -ne 0 -or $tagPushCode -ne 0) {
    Write-Warn "Push may have encountered issues. Check remote status."
}
Write-Success "Pushed commits and tag"

# ── Summary ─────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Release v$Version created successfully!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Info "GitHub Actions will now build and publish the release."
Write-Info "Check progress at: https://github.com/Swatto86/EventSleuth/actions"
Write-Host ""
