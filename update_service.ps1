# SmartCard Service Update Script
$ErrorActionPreference = "Stop"

$currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Start-Process powershell.exe -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`"" -Verb RunAs
    exit
}

Write-Host "==============================================================" -ForegroundColor Cyan
Write-Host "  SmartCard RTS5169 Service Update (PowerShell Native)" -ForegroundColor Cyan
Write-Host "==============================================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "[1/3] Stopping SmartCardUnlock service..." -ForegroundColor Yellow
Stop-Service -Name "SmartCardUnlock" -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 1000

Write-Host "[2/4] Updating core binaries..." -ForegroundColor Yellow
$src = Join-Path $PSScriptRoot "rust-core\target\release\smartcard_unlock.exe"
$dst = Join-Path $PSScriptRoot "bin\smartcard_unlock.exe"
Copy-Item -Path $src -Destination $dst -Force

Write-Host "[3/4] Building and deploying Credential Provider COM DLL..." -ForegroundColor Yellow
$buildScript = Join-Path $PSScriptRoot "build_credprov.bat"
& $buildScript
$dllSrc = Join-Path $PSScriptRoot "bin\SmartCardCredentialProvider.dll"
$progDataDir = "C:\ProgramData\SmartCardUnlock"
$dllDst = Join-Path $progDataDir "SmartCardCredentialProvider.dll"
if (Test-Path $dllSrc) {
    if (-not (Test-Path $progDataDir)) { New-Item -ItemType Directory -Path $progDataDir -Force | Out-Null }
    Copy-Item -Path $dllSrc -Destination $dllDst -Force
    Start-Process -FilePath "regsvr32.exe" -ArgumentList "/s `"$dllDst`"" -Wait
}

Write-Host "[4/4] Starting SmartCardUnlock service..." -ForegroundColor Yellow
Start-Service -Name "SmartCardUnlock"

Write-Host ""
Write-Host "[OK] Service updated and running the latest build!" -ForegroundColor Green
Write-Host ""
& (Join-Path $PSScriptRoot "bin\smartcard_unlock.exe") status
Write-Host ""
Start-Sleep -Seconds 2
