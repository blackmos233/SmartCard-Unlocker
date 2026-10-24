@echo off
title SmartCard Unlock - Service Installer

net session >nul 2>&1
if errorlevel 1 (
    echo [UAC] Requesting Administrator Privileges...
    powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b
)

cd /d "%~dp0"
echo ==============================================================
echo   SmartCard RTS5169 Service Installation
echo ==============================================================
echo.

"%~dp0bin\smartcard_unlock.exe" install-service

echo.
echo Current Status:
"%~dp0bin\smartcard_unlock.exe" status

echo.
echo ==============================================================
echo [OK] Installation completed! Auto-start enabled.
echo ==============================================================
echo.
pause
