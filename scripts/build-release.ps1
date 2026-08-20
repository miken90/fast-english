# Fast English - Release Build Script
# Run from the project root directory.
#
# Usage:
#   .\scripts\build-release.ps1              portable only (the default scope)
#   .\scripts\build-release.ps1 -Installer   also compile the Inno Setup installer
#
# ASCII only. Without a BOM, PowerShell 5.1 reads this file as ANSI, where a
# UTF-8 em dash decodes to a curly quote that silently terminates a string.
#
# Every step is checked against what it actually produced. $ErrorActionPreference
# is deliberately not "Stop": cargo writes progress to stderr, which under Stop
# aborts the script mid-run while still leaving exit code 0 - the failure mode
# this script exists to make impossible. The assertions below are the contract.

param(
    # Installer release is opt-in; portable is the standard scope.
    [switch]$Installer,
    # The zip is named from Cargo.toml's version, so a rebuild at an unchanged
    # version would otherwise overwrite an already-published zip. -Force opts
    # into that overwrite deliberately.
    [switch]$Force
)

# Artifacts older than this are left over from an earlier build, not this one.
$runStart = Get-Date

function Fail {
    param([string]$Message)
    Write-Host "FAILED: $Message" -ForegroundColor Red
    exit 1
}

# Exists and has content. Used for cargo's own output, which is legitimately
# left untouched when nothing needed rebuilding.
function Assert-Artifact {
    param([string]$Path, [string]$Label)

    if (!(Test-Path -LiteralPath $Path)) {
        Fail "$Label was not produced ($Path)"
    }
    $item = Get-Item -LiteralPath $Path
    if ($item.Length -le 0) {
        Fail "$Label is empty ($Path)"
    }
    $mb = [math]::Round($item.Length / 1MB, 2)
    Write-Host "  OK  $Label - $mb MB" -ForegroundColor Green
}

# Adds the mtime check. Used for the artifacts this script rewrites every run,
# where an old timestamp means the step that should have replaced it failed.
function Assert-FreshArtifact {
    param([string]$Path, [string]$Label)

    if (!(Test-Path -LiteralPath $Path)) {
        Fail "$Label was not produced ($Path)"
    }
    $item = Get-Item -LiteralPath $Path
    if ($item.Length -le 0) {
        Fail "$Label is empty ($Path)"
    }
    if ($item.LastWriteTime -lt $runStart) {
        $stamp = $item.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss')
        Fail "$Label is stale - last written $stamp, before this run started. A previous build's artifact is still on disk; the step that should have replaced it did not run."
    }
    $mb = [math]::Round($item.Length / 1MB, 2)
    $stamp = $item.LastWriteTime.ToString('HH:mm:ss')
    Write-Host "  OK  $Label - $mb MB, $stamp" -ForegroundColor Green
}

Write-Host "Building Fast English release..." -ForegroundColor Cyan

# Cargo.toml is the single source of truth for the version.
$cargoToml = Get-Content -LiteralPath "Cargo.toml" -Raw
if ($cargoToml -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    Fail "could not read the package version from Cargo.toml"
}
$version = $Matches[1]
Write-Host "Version from Cargo.toml: $version" -ForegroundColor Cyan

# --- Build ---
cargo build --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) {
    Fail "cargo build exited with $LASTEXITCODE"
}

$binary = "target\x86_64-pc-windows-msvc\release\fast-english.exe"
Assert-Artifact -Path $binary -Label "release binary"

# --- Copy to dist ---
$dist = "dist"
if (!(Test-Path -LiteralPath $dist)) {
    New-Item -ItemType Directory -Path $dist | Out-Null
}

$distExe = Join-Path $dist "FastEnglish.exe"
Copy-Item -LiteralPath $binary -Destination $distExe -Force -ErrorAction Continue

# Not an mtime check: Copy-Item preserves the source timestamp, so a copied
# file never looks newer than the build that produced it. Byte-identity with
# the binary from this run is the stronger claim anyway - it fails exactly when
# a stale or partial exe is sitting in dist, which is the case that matters.
Assert-Artifact -Path $distExe -Label "dist\FastEnglish.exe"
$builtHash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
$distHash = (Get-FileHash -LiteralPath $distExe -Algorithm SHA256).Hash
if ($builtHash -ne $distHash) {
    Fail "dist\FastEnglish.exe does not match the binary just built - the copy did not land, so an older exe is still in dist"
}
Write-Host "  OK  dist\FastEnglish.exe matches the built binary (SHA256 $($distHash.Substring(0,16))...)" -ForegroundColor Green

# The embedded version comes from Cargo.toml via build.rs; a mismatch means a
# stale binary slipped through or the resource stopped being embedded.
$embedded = (Get-Item -LiteralPath $distExe).VersionInfo.ProductVersion
if ($embedded -ne $version) {
    Fail "dist\FastEnglish.exe reports version '$embedded' but Cargo.toml says '$version'"
}
Write-Host "  OK  embedded version matches Cargo.toml ($embedded)" -ForegroundColor Green

# --- Portable zip ---
$zipName = "FastEnglish-$version-portable.zip"
$zipPath = Join-Path $dist $zipName

# The zip name is derived from Cargo.toml's version alone, so a rebuild at an
# unchanged version would silently overwrite an already-published zip. Refuse
# unless the caller explicitly opts in with -Force.
if ((Test-Path -LiteralPath $zipPath) -and -not $Force) {
    Fail "dist\$zipName already exists and Cargo.toml's version ($version) has not changed. Refusing to overwrite a possibly-published zip. Bump the version in Cargo.toml, or rerun with -Force to overwrite deliberately."
}

Compress-Archive -LiteralPath $distExe -DestinationPath $zipPath -Force -ErrorAction Continue
Assert-FreshArtifact -Path $zipPath -Label "dist\$zipName"

# --- Installer (opt-in) ---
if ($Installer) {
    Write-Host "Compiling installer..." -ForegroundColor Cyan

    # The .iss carries no version of its own and errors out without this define.
    # Printed before the toolchain lookup so the value is auditable even where
    # Inno Setup is not installed.
    $define = "/DAppVersion=$version"
    Write-Host "  ISCC define: $define" -ForegroundColor Cyan

    $iscc = @(
        "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
        "C:\Program Files\Inno Setup 6\ISCC.exe"
    ) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if (-not $iscc) {
        $onPath = Get-Command iscc.exe -ErrorAction SilentlyContinue
        if ($onPath) { $iscc = $onPath.Source }
    }
    if (-not $iscc) {
        Fail "Inno Setup 6 (ISCC.exe) was not found. Install it, or omit -Installer for a portable-only release."
    }

    & $iscc $define "installer\FastEnglish.iss"
    if ($LASTEXITCODE -ne 0) {
        Fail "ISCC exited with $LASTEXITCODE"
    }

    Assert-FreshArtifact -Path (Join-Path $dist "FastEnglish-Setup-$version.exe") -Label "dist\FastEnglish-Setup-$version.exe"
}

Write-Host ""
Write-Host "Done! Distribution files in $dist\" -ForegroundColor Cyan
exit 0
