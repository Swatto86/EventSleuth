<#
.SYNOPSIS
    Automated release pipeline for EventSleuth.

.DESCRIPTION
    Automates the full release lifecycle per Rule 18 of the Engineering Contract:
    version validation, manifest updates, pre-release build and quality gates,
    git commit, tag, push, and cleanup of older release tags.

    On failure at any step, the script rolls back version changes and aborts.

.PARAMETER Version
    Target semantic version (x.y.z). Prompted interactively when omitted.

.PARAMETER Notes
    Release notes text. Prompted interactively when omitted (multi-line,
    terminated by a blank line).

.PARAMETER Force
    Allow overwriting an existing tag or releasing without a version increment.

.PARAMETER DryRun
    Describe every planned action without modifying files, git, or remote hosting.

.EXAMPLE
    .\update-application.ps1
    # Interactive mode: prompts for version and notes.

.EXAMPLE
    .\update-application.ps1 -Version "1.1.0" -Notes "Added feature X"
    # Parameterised mode: no prompts.

.EXAMPLE
    .\update-application.ps1 -Version "1.0.4" -Notes "Hotfix" -Force
    # Force mode: skip duplicate version check.

.EXAMPLE
    .\update-application.ps1 -Version "1.1.0" -Notes "Test" -DryRun
    # Dry-run mode: print planned actions without making changes.
#>

[CmdletBinding()]
param(
    [string]$Version,
    [string]$Notes,
    [switch]$Force,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

# ── Coloured output helpers ─────────────────────────────────────────────

function Write-Info      { param($msg) Write-Host "  [INFO] $msg" -ForegroundColor Cyan }
function Write-Success   { param($msg) Write-Host "  [OK]   $msg" -ForegroundColor Green }
function Write-WarnLine  { param($msg) Write-Host "  [WARN] $msg" -ForegroundColor Yellow }
function Write-ErrorLine { param($msg) Write-Host "  [ERR]  $msg" -ForegroundColor Red }

# ── Git helpers ─────────────────────────────────────────────────────────

function Invoke-Git {
    <#
    .SYNOPSIS
        Run a git command and throw on non-zero exit code.

    .NOTES
        Intentionally a simple (non-advanced) function with no [Parameter()] or
        [CmdletBinding()] so that PowerShell never adds common parameters.
        Advanced functions add -Debug, -Verbose, etc. as abbreviatable names,
        which means a call like `Invoke-Git tag -d "v1.0.0"` would bind `-d`
        to `-Debug` (a valid abbreviation) rather than forwarding it to git,
        causing `tag -d` to silently become `tag` (create instead of delete).
        Using the automatic $args variable avoids all named-parameter binding.
    #>
    $output = & git @args 2>&1
    if ($LASTEXITCODE -ne 0) {
        $message = ($output | Out-String).Trim()
        throw "git $($args -join ' ') failed (exit $LASTEXITCODE): $message"
    }
    return $output
}

function Test-IsGitRepository {
    try {
        $null = Invoke-Git rev-parse --is-inside-work-tree
        return $true
    } catch {
        return $false
    }
}

function Get-RemoteHttpsUrl {
    try {
        $url = (Invoke-Git remote get-url origin) | Out-String
        $url = $url.Trim()
        if ($url -match '^git@([^:]+):(.+?)(?:\.git)?$') {
            $url = "https://$($Matches[1])/$($Matches[2])"
        }
        $url = $url -replace '\.git$', ''
        return $url
    } catch {
        return "https://github.com/Swatto86/EventSleuth"
    }
}

# ── Path resolution ─────────────────────────────────────────────────────

function Get-WorkspaceRoot {
    return (Split-Path -Parent $PSCommandPath)
}

# ── Version helpers ─────────────────────────────────────────────────────

function Get-PackageVersion {
    <#
    .SYNOPSIS
        Read the current version from Cargo.toml using regex.
    #>
    param([string]$ManifestPath)
    $content = Get-Content $ManifestPath -Raw
    $match = [regex]::Match($content, '(?m)^version = "([^"]+)"')
    if (-not $match.Success) {
        throw "Could not parse current version from $ManifestPath"
    }
    return $match.Groups[1].Value
}

function Compare-SemVer {
    <#
    .SYNOPSIS
        Three-component numeric comparison returning -1, 0, or 1.
    #>
    param([string]$Left, [string]$Right)
    $lParts = $Left -split '\.'
    $rParts = $Right -split '\.'
    for ($i = 0; $i -lt 3; $i++) {
        $l = [int]$lParts[$i]
        $r = [int]$rParts[$i]
        if ($l -lt $r) { return -1 }
        if ($l -gt $r) { return  1 }
    }
    return 0
}

function Update-PackageVersion {
    <#
    .SYNOPSIS
        Regex-replace version in Cargo.toml, preserving line endings.
        Writes UTF-8 without BOM.
    #>
    param([string]$ManifestPath, [string]$NewVersion)

    $raw = [System.IO.File]::ReadAllText($ManifestPath)

    # Detect line-ending style
    $useCRLF = $raw.Contains("`r`n")

    # Replace version string
    $updated = [regex]::Replace($raw, '(?m)^version = "[^"]+"', "version = `"$NewVersion`"")

    # Normalise to exactly one trailing newline in the detected style
    $updated = $updated.TrimEnd()
    if ($useCRLF) {
        $updated += "`r`n"
    } else {
        $updated += "`n"
    }

    # Write UTF-8 without BOM
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($ManifestPath, $updated, $utf8NoBom)
}

# ── Main body ───────────────────────────────────────────────────────────

$workspaceRoot = Get-WorkspaceRoot
Set-Location $workspaceRoot

Write-Host ""
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   EventSleuth Release Pipeline"         -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta

$cargoToml = Join-Path $workspaceRoot "Cargo.toml"
$cargoLock = Join-Path $workspaceRoot "Cargo.lock"

if (-not (Test-Path $cargoToml)) {
    Write-ErrorLine "Cargo.toml not found at $cargoToml"
    exit 1
}

$currentVersion = Get-PackageVersion -ManifestPath $cargoToml
Write-Info "Current version: $currentVersion"

# ── 1. Collect & validate version ───────────────────────────────────────

if (-not $Version) {
    $Version = Read-Host "Enter the new version (semver, e.g. 1.1.0)"
}

if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    Write-ErrorLine "Invalid version format '$Version'. Must be semver: MAJOR.MINOR.PATCH"
    exit 1
}

if ($Version -eq $currentVersion -and -not $Force) {
    Write-ErrorLine "Version $Version is the same as current. Use -Force to override."
    exit 1
}

$cmp = Compare-SemVer -Left $Version -Right $currentVersion
if ($cmp -lt 0 -and -not $Force) {
    Write-ErrorLine "Version $Version is lower than current $currentVersion. Use -Force to override."
    exit 1
}

Write-Success "Version $Version validated (current: $currentVersion)"

# ── 2. Collect & validate release notes ─────────────────────────────────

if (-not $Notes) {
    Write-Host ""
    Write-Host "Enter release notes (what changed in this version)." -ForegroundColor Cyan
    Write-Host "These notes will appear on the GitHub Releases page." -ForegroundColor Yellow
    Write-Host "Enter a blank line when done:" -ForegroundColor Gray
    Write-Host ""

    $noteLines = @()
    while ($true) {
        $line = Read-Host
        if ([string]::IsNullOrWhiteSpace($line)) { break }
        $noteLines += $line
    }
    $Notes = $noteLines -join "`n"
}

if (-not $Notes -or $Notes.Trim() -eq '') {
    Write-ErrorLine "Release notes must not be empty."
    exit 1
}

# ── 3. Git state checks ────────────────────────────────────────────────

if (-not $DryRun) {
    if (-not (Test-IsGitRepository)) {
        Write-ErrorLine "Not inside a git repository."
        exit 1
    }
}

# Check for existing tag
$existingTag = $null
try {
    $existingTag = Invoke-Git tag -l "v$Version"
} catch {
    # Ignore -- may not be in a repo during DryRun
}
if ($existingTag -and -not $Force) {
    Write-ErrorLine "Tag v$Version already exists. Use -Force to overwrite."
    exit 1
}

# Warn on dirty working tree
try {
    $gitStatus = Invoke-Git status --porcelain
    if ($gitStatus) {
        Write-WarnLine "Uncommitted changes detected in working tree."
    }
} catch {
    if (-not $DryRun) { throw }
}

# ── 4. Snapshot originals for rollback ──────────────────────────────────

$cargoTomlOriginal = [System.IO.File]::ReadAllText($cargoToml)
$cargoLockOriginal = $null
if (Test-Path $cargoLock) {
    $cargoLockOriginal = [System.IO.File]::ReadAllText($cargoLock)
}

# ── 5. Dry-run mode ────────────────────────────────────────────────────

if ($DryRun) {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Yellow
    Write-Host "  DRY RUN -- No changes will be made"    -ForegroundColor Yellow
    Write-Host "========================================" -ForegroundColor Yellow
    Write-Host ""
    Write-Info "Release summary:"
    Write-Host "  Current version : $currentVersion"
    Write-Host "  New version     : $Version"
    Write-Host "  Tag             : v$Version"
    Write-Host ""
    Write-Info "Release notes:"
    Write-Host "---"
    Write-Host $Notes
    Write-Host "---"
    Write-Host ""
    Write-Info "Planned actions:"
    Write-Host "  1. Update version in Cargo.toml: $currentVersion -> $Version"
    Write-Host "  2. Run: cargo update (refresh Cargo.lock)"
    Write-Host "  3. Run: cargo build --release"
    Write-Host "  4. Run: cargo fmt -- --check"
    Write-Host "  5. Run: cargo clippy -- -D warnings"
    Write-Host "  6. Run: cargo test"
    if ($existingTag -and $Force) {
        Write-Host "  7. Delete existing tag v$Version (local + remote)"
    }
    Write-Host "  8. git add Cargo.toml Cargo.lock"
    Write-Host "  9. git commit -m 'chore: bump version to $Version'"
    Write-Host " 10. git tag -a v$Version -m <notes>"
    Write-Host " 11. git push origin HEAD"
    Write-Host " 12. git push origin v$Version"
    Write-Host " 13. Prune older release tags (all v*.*.* except v$Version)"
    Write-Host ""
    Write-Success "Dry run complete. No changes were made."
    exit 0
}

# ── Step 1: Update version strings ──────────────────────────────────────

Write-Host ""
Write-Host "=== Step 1: Update version strings ===" -ForegroundColor Magenta

try {
    Update-PackageVersion -ManifestPath $cargoToml -NewVersion $Version
    Write-Success "Updated Cargo.toml: $currentVersion -> $Version"

    Write-Info "Running cargo update to refresh Cargo.lock..."
    $null = & cargo update 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-WarnLine "cargo update returned non-zero exit (non-blocking)"
    }
    Write-Success "Cargo.lock refreshed"
} catch {
    Write-ErrorLine "Failed to update version: $_"
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($cargoToml, $cargoTomlOriginal, $utf8NoBom)
    exit 1
}

$changedFiles = @($cargoToml)
if (Test-Path $cargoLock) { $changedFiles += $cargoLock }

# ── Show summary and diff, then confirm ─────────────────────────────────

Write-Host ""
Write-Info "Release summary:"
Write-Host "  Current version : $currentVersion"
Write-Host "  New version     : $Version"
Write-Host "  Tag             : v$Version"
Write-Host ""
Write-Info "Release notes:"
Write-Host "---"
Write-Host $Notes
Write-Host "---"
Write-Host ""

Write-Info "Changes:"
try {
    foreach ($f in $changedFiles) {
        $relPath = [System.IO.Path]::GetRelativePath($workspaceRoot, $f)
        $diff = & git diff -- $relPath 2>&1
        if ($diff) {
            Write-Host $($diff | Out-String)
        }
    }
} catch {
    Write-WarnLine "Could not display git diff."
}

if (-not $Force) {
    $confirm = Read-Host "Proceed? (y/N)"
    if ($confirm -notmatch '^[yY](es|ES)?$') {
        Write-WarnLine "Aborted by user. Rolling back..."
        $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
        [System.IO.File]::WriteAllText($cargoToml, $cargoTomlOriginal, $utf8NoBom)
        if ($cargoLockOriginal -and (Test-Path $cargoLock)) {
            [System.IO.File]::WriteAllText($cargoLock, $cargoLockOriginal, $utf8NoBom)
        }
        try { Invoke-Git checkout -- $cargoToml } catch { }
        exit 1
    }
}

# ── Wrap remaining steps in try/catch for rollback ──────────────────────

try {

    # ── Step 2: Pre-release build ───────────────────────────────────────

    Write-Host ""
    Write-Host "=== Step 2: Pre-release build ===" -ForegroundColor Magenta

    Write-Info "Building release configuration..."
    $buildOutput = & cargo build --release 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Release build failed: $($buildOutput | Out-String)"
    }
    Write-Success "Release build succeeded"

    # ── Step 3: Quality gates ───────────────────────────────────────────

    Write-Host ""
    Write-Host "=== Step 3: Quality gates ===" -ForegroundColor Magenta

    Write-Info "Checking formatting (cargo fmt)..."
    $fmtOutput = & cargo fmt -- --check 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Format check failed. Run 'cargo fmt' first: $($fmtOutput | Out-String)"
    }
    Write-Success "Format check passed"

    Write-Info "Running clippy..."
    $clippyOutput = & cargo clippy -- -D warnings 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Clippy lint failed: $($clippyOutput | Out-String)"
    }
    Write-Success "Clippy clean"

    Write-Info "Running full test suite..."
    $testOutput = & cargo test 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Tests failed: $($testOutput | Out-String)"
    }
    Write-Success "All tests passed"

    # ── Step 4: Handle existing tag ─────────────────────────────────────

    if ($Force -and $existingTag) {
        Write-Host ""
        Write-Host "=== Step 4: Handle existing tag ===" -ForegroundColor Magenta

        Write-Info "Deleting existing local tag v$Version..."
        try { Invoke-Git tag -d "v$Version" } catch { Write-WarnLine "Local tag deletion: $_" }

        Write-Info "Deleting existing remote tag v$Version..."
        $ErrorActionPreference = "Continue"
        & git push origin --delete "v$Version" 2>&1 | Out-Null
        $ErrorActionPreference = "Stop"
        Write-Success "Existing tag v$Version removed"
    }

    # ── Step 5: Commit version bump ─────────────────────────────────────

    Write-Host ""
    Write-Host "=== Step 5: Commit version bump ===" -ForegroundColor Magenta

    Invoke-Git add $cargoToml
    if (Test-Path $cargoLock) { Invoke-Git add $cargoLock }

    # Check whether there is anything staged
    $staged = Invoke-Git diff --cached --name-only
    if (-not $staged) {
        Write-WarnLine "No changes to commit (version may already be $Version in git)"
    } else {
        Invoke-Git commit -m "chore: bump version to $Version"
        Write-Success "Committed version bump"
    }

    # ── Step 6: Tag and push ────────────────────────────────────────────

    Write-Host ""
    Write-Host "=== Step 6: Tag and push ===" -ForegroundColor Magenta

    $tempFile = [System.IO.Path]::GetTempFileName()
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($tempFile, $Notes, $utf8NoBom)
    Invoke-Git tag -a "v$Version" -F $tempFile
    Remove-Item $tempFile -ErrorAction SilentlyContinue
    Write-Success "Created tag v$Version"

    Write-Info "Pushing commits..."
    Invoke-Git push origin HEAD
    Write-Success "Pushed commits"

    Write-Info "Pushing tag..."
    Invoke-Git push origin "v$Version"
    Write-Success "Pushed tag v$Version"

    # ── Step 7: Prune older release tags ────────────────────────────────

    Write-Host ""
    Write-Host "=== Step 7: Prune older release tags ===" -ForegroundColor Magenta

    $allTags = @()
    try { $allTags = @(Invoke-Git tag -l "v*.*.*") } catch { }

    $ghAvailable = $null -ne (Get-Command gh -ErrorAction SilentlyContinue)
    $pruneCount = 0

    foreach ($tag in $allTags) {
        $tag = "$tag".Trim()
        if (-not $tag -or $tag -eq "v$Version") { continue }

        # Delete local tag
        try { Invoke-Git tag -d $tag } catch { }

        # Delete remote tag
        $ErrorActionPreference = "Continue"
        & git push origin --delete $tag 2>&1 | Out-Null
        $ErrorActionPreference = "Stop"

        # Delete GitHub Release if gh CLI is available
        if ($ghAvailable) {
            $ErrorActionPreference = "Continue"
            & gh release delete $tag --yes 2>&1 | Out-Null
            $ErrorActionPreference = "Stop"
        }

        $pruneCount++
    }

    if ($pruneCount -gt 0) {
        Write-Success "Pruned $pruneCount older release tag(s)"
    } else {
        Write-Info "No older tags to prune"
    }

    # ── Done ────────────────────────────────────────────────────────────

    $repoUrl = Get-RemoteHttpsUrl

    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "  Release v$Version created successfully!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    Write-Info "GitHub Actions will now build and publish the release."
    Write-Info "Check progress at: $repoUrl/actions"
    Write-Host ""

} catch {
    # ── Rollback on failure ─────────────────────────────────────────────
    Write-Host ""
    Write-ErrorLine "Release failed: $_"
    Write-WarnLine "Rolling back manifest changes..."

    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($cargoToml, $cargoTomlOriginal, $utf8NoBom)
    if ($cargoLockOriginal -and (Test-Path $cargoLock)) {
        [System.IO.File]::WriteAllText($cargoLock, $cargoLockOriginal, $utf8NoBom)
    }

    Write-WarnLine "Rollback complete."
    exit 1
}
