# Build chromeless for Windows
# Requires: Rust 1.75+ (install from https://rustup.rs)
# WebView2 Runtime is pre-installed on Windows 11.
# For Windows 10, ensure the Evergreen Runtime is installed:
#   https://developer.microsoft.com/en-us/microsoft-edge/webview2/

$ErrorActionPreference = "Stop"

Write-Host "▸ building chromeless" -ForegroundColor Cyan
cargo build --release

if ($LASTEXITCODE -ne 0) {
    Write-Host "✗ build failed" -ForegroundColor Red
    exit 1
}

$binary = "target\release\chromeless.exe"
if (Test-Path $binary) {
    $size = (Get-Item $binary).Length
    $mb = [math]::Round($size / 1MB, 2)
    Write-Host "✓ built $binary ($mb MB)" -ForegroundColor Green
    Write-Host "  try:  .\$binary --help"
} else {
    Write-Host "✗ binary not found at $binary" -ForegroundColor Red
    exit 1
}
