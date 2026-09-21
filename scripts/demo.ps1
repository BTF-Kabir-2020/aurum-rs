# scripts/demo.ps1 — one-command showcase for Windows (README §30, OPERATIONS).
# Usage: .\scripts\demo.ps1 [-Speed 10] | -Help
param(
    [int]$Speed = 10,
    [switch]$Help
)
$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Output "demo.ps1 - Aurum offline showcase (no API key, no network)"
    Write-Output "Usage: .\scripts\demo.ps1 [-Speed 1..10]"
    exit 0
}

$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root "target\release\aurum.exe"

if (-not (Test-Path $exe)) {
    Write-Host "building release binary (first run only)..." -ForegroundColor Cyan
    & cargo build --release --quiet
    if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }
}

Set-Location $root
& $exe version
& $exe demo --speed $Speed
Write-Output ""
& $exe doctor
Write-Output ""
Write-Output "Try next:  .\target\release\aurum.exe server   # API at http://127.0.0.1:8080/api/v1/*"
