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

set DLL_SRC=bin\SmartCardCredentialProvider.dll
set DEST_DIR=C:\ProgramData\SmartCardUnlock
set DLL_DEST=%DEST_DIR%\SmartCardCredentialProvider.dll

if not exist "%DLL_SRC%" (
    echo [INFO] DLL not found, compiling now...
    call build_credprov.bat
    if not exist "%DLL_SRC%" (
        echo [ERROR] Build failed, aborting registration.
        pause
        exit /b 1
    )
)

if not exist "%DEST_DIR%" mkdir "%DEST_DIR%"

echo [INFO] Copying DLL to %DEST_DIR%...
copy /y "%DLL_SRC%" "%DLL_DEST%" >nul
if %ERRORLEVEL% NEQ 0 (
    echo [WARNING] Direct copy failed, attempting to unregister existing DLL first...
    regsvr32.exe /u /s "%DLL_DEST%"
    copy /y "%DLL_SRC%" "%DLL_DEST%" >nul
)

echo [INFO] Registering Credential Provider with regsvr32...
regsvr32.exe /s "%DLL_DEST%"
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] regsvr32 failed with code: %ERRORLEVEL%
    pause
    exit /b %ERRORLEVEL%
)

echo [INFO] Verifying registration in Windows Registry...
reg query "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\{7C3B984A-94B6-4E87-8A33-8FE58D6E9A42}" >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo [SUCCESS] SmartCardCredentialProvider registered successfully!
    echo [INFO] Provider CLSID: {7C3B984A-94B6-4E87-8A33-8FE58D6E9A42}
    echo [INFO] DLL Path: %DLL_DEST%
) else (
    echo [ERROR] Registration verification failed!
)

echo.
pause
exit /b 0
