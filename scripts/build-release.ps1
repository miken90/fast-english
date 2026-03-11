# ToneWriter — Release Build Script
# Run from the project root directory

$ErrorActionPreference = "Stop"

Write-Host "Building ToneWriter release..." -ForegroundColor Cyan

# Build release binary
cargo build --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

$binary = "target\x86_64-pc-windows-msvc\release\tone-writer.exe"
$size = (Get-Item $binary).Length / 1MB
Write-Host "Build complete: $binary ($([math]::Round($size, 1)) MB)" -ForegroundColor Green

# Create dist folder
$dist = "dist"
if (!(Test-Path $dist)) { New-Item -ItemType Directory -Path $dist | Out-Null }

# Copy binary to dist
Copy-Item $binary "$dist\ToneWriter.exe" -Force
Write-Host "Copied to $dist\ToneWriter.exe" -ForegroundColor Green

# Create portable zip
$version = "1.0.0"
$zipName = "ToneWriter-$version-portable.zip"
Compress-Archive -Path "$dist\ToneWriter.exe" -DestinationPath "$dist\$zipName" -Force
Write-Host "Portable zip: $dist\$zipName" -ForegroundColor Green

Write-Host "`nDone! Distribution files in $dist\" -ForegroundColor Cyan
