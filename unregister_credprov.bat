@echo off
setlocal EnableDelayedExpansion

:: Administrator Check & Auto Elevation
net session >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [INFO] Requesting Administrator privileges...
    powershell -Command "Start-Process cmd -ArgumentList '/c \"\"%~f0\"\"' -Verb RunAs"
    exit /b 0
)

cd /d "%~dp0"

set DEST_DIR=C:\ProgramData\SmartCardUnlock
set DLL_DEST=%DEST_DIR%\SmartCardCredentialProvider.dll

echo [INFO] Unregistering SmartCardCredentialProvider...

if exist "%DLL_DEST%" (
    regsvr32.exe /u /s "%DLL_DEST%"
)

:: Ensure registry keys are removed
reg delete "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\{7C3B984A-94B6-4E87-8A33-8FE58D6E9A42}" /f >nul 2>&1
reg delete "HKLM\SOFTWARE\Classes\CLSID\{7C3B984A-94B6-4E87-8A33-8FE58D6E9A42}" /f >nul 2>&1

echo [SUCCESS] SmartCardCredentialProvider unregistered successfully!
echo.
pause
exit /b 0
