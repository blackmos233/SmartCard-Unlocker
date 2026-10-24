# Register SmartCard Credential Provider
$ErrorActionPreference = "Stop"

$currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Start-Process powershell.exe -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`"" -Verb RunAs
    exit
}

Write-Host "==============================================================" -ForegroundColor Cyan
Write-Host "  Register SmartCard Credential Provider" -ForegroundColor Cyan
Write-Host "==============================================================" -ForegroundColor Cyan
Write-Host ""

$dllSrc = Join-Path $PSScriptRoot "bin\SmartCardCredentialProvider.dll"
$destDir = "C:\ProgramData\SmartCardUnlock"
$dllDest = Join-Path $destDir "SmartCardCredentialProvider.dll"

if (-not (Test-Path $dllSrc)) {
    Write-Host "[INFO] DLL not found, compiling now..." -ForegroundColor Yellow
    & (Join-Path $PSScriptRoot "build_credprov.bat")
}

if (-not (Test-Path $destDir)) {
    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
}

Write-Host "[1/3] Copying DLL to $destDir..." -ForegroundColor Yellow
Copy-Item -Path $dllSrc -Destination $dllDest -Force

Write-Host "[2/3] Registering Credential Provider with regsvr32..." -ForegroundColor Yellow
$proc = Start-Process -FilePath "regsvr32.exe" -ArgumentList "/s `"$dllDest`"" -Wait -PassThru
if ($proc.ExitCode -ne 0) {
    Write-Host "[ERROR] regsvr32 failed with exit code: $($proc.ExitCode)" -ForegroundColor Red
    pause
    exit $proc.ExitCode
}

Write-Host "[3/3] Verifying registration in Windows Registry..." -ForegroundColor Yellow
$regKey = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\{7C3B984A-94B6-4E87-8A33-8FE58D6E9A42}"
if (Test-Path $regKey) {
    Write-Host ""
    Write-Host "[SUCCESS] SmartCardCredentialProvider registered successfully!" -ForegroundColor Green
    Write-Host "  Provider CLSID: {7C3B984A-94B6-4E87-8A33-8FE58D6E9A42}" -ForegroundColor Green
    Write-Host "  DLL Path: $dllDest" -ForegroundColor Green
} else {
    Write-Host "[ERROR] Registry key verification failed!" -ForegroundColor Red
}

Write-Host ""
pause
