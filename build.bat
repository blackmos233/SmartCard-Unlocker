@echo off
setlocal EnableDelayedExpansion

echo ==============================================================
echo   SmartCard Auto Unlock - Unified Build System
echo ==============================================================
echo.

cd /d "%~dp0"
if not exist bin mkdir bin

:: 1. Detect MinGW g++ compiler
echo [1/2] Compiling Windows Credential Provider COM DLL (C++)...
set GPP=g++
where g++ >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    if exist "C:\mingw64\bin\g++.exe" (
        set GPP=C:\mingw64\bin\g++.exe
    ) else if exist "D:\work\path\mingw64\bin\g++.exe" (
        set GPP=D:\work\path\mingw64\bin\g++.exe
    ) else (
        echo [ERROR] MinGW-w64 g++ compiler not found in PATH!
        echo Please ensure MinGW-w64 is installed and added to PATH.
        exit /b 1
    )
)

"%GPP%" -shared -O2 -static ^
    -D_WIN32_WINNT=0x0A00 -DNTDDI_VERSION=0x0A000002 ^
    -o bin\SmartCardCredentialProvider.dll ^
    credential_provider\dllmain.cpp ^
    credential_provider\CClassFactory.cpp ^
    credential_provider\CSmartCardProvider.cpp ^
    credential_provider\CSmartCardCredential.cpp ^
    credential_provider\SmartCardCredentialProvider.def ^
    -luuid -lole32 -lsecur32 -lwinscard -lcrypt32 -lwtsapi32 -lshlwapi -ladvapi32

if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Failed to compile SmartCardCredentialProvider.dll!
    exit /b %ERRORLEVEL%
)
echo [OK] bin\SmartCardCredentialProvider.dll built successfully.
echo.

:: 2. Detect Cargo and compile Rust Core
echo [2/2] Compiling Core System Service (Rust)...
where cargo >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Cargo / Rust toolchain not found in PATH!
    exit /b 1
)

cd rust-core
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Cargo build failed!
    cd ..
    exit /b %ERRORLEVEL%
)
cd ..

copy /y "rust-core\target\release\smartcard_unlock.exe" "bin\smartcard_unlock.exe" >nul
if %ERRORLEVEL% NEQ 0 (
    echo [WARNING] Direct overwrite of bin\smartcard_unlock.exe failed. File may be locked by running service.
) else (
    echo [OK] bin\smartcard_unlock.exe built and updated successfully.
)

echo.
echo ==============================================================
echo [SUCCESS] All components built successfully!
echo ==============================================================
dir bin\*.exe bin\*.dll | findstr /i "smartcard"
echo.
exit /b 0
