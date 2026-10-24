@echo off
title SmartCard Unlock - Service Uninstaller

net session >nul 2>&1
if errorlevel 1 (
    echo [UAC] Requesting Administrator Privileges...
    powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b
)

cd /d "%~dp0"
echo ==============================================================
echo   SmartCard RTS5169 Service Uninstaller
echo ==============================================================
echo.

"%~dp0bin\smartcard_unlock.exe" uninstall-service

echo.
pause
