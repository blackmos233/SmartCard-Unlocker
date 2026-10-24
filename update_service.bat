@echo off
title SmartCard Unlock - Service Update

net session >nul 2>&1
if errorlevel 1 (
    echo [UAC] Requesting Administrator Privileges...
    powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b
)

cd /d "%~dp0"
echo ==============================================================
echo   SmartCard RTS5169 Service Update
echo ==============================================================
echo.

echo [1/3] Stopping SmartCardUnlock service...
net stop SmartCardUnlock >nul 2>&1
timeout /t 1 /nobreak >nul

echo [2/3] Updating core binary...
copy /y "%~dp0rust-core\target\release\smartcard_unlock.exe" "%~dp0bin\smartcard_unlock.exe"
if errorlevel 1 (
    echo.
    echo [ERROR] Failed to update binary! Please check if files are locked.
    echo.
    pause
    exit /b
)

echo [3/3] Starting SmartCardUnlock service...
net start SmartCardUnlock

echo.
echo ==============================================================
echo [OK] Update completed! Service is now running the latest build.
echo ==============================================================
echo.
"%~dp0bin\smartcard_unlock.exe" status
echo.
pause
