@echo off
setlocal

set GPP=D:\work\path\mingw64\bin\g++.exe
if not exist "%GPP%" set GPP=g++

if not exist bin mkdir bin

echo [BUILD] Compiling SmartCardCredentialProvider.dll...
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
    echo [ERROR] Build failed! Error code: %ERRORLEVEL%
    exit /b %ERRORLEVEL%
)

echo [SUCCESS] SmartCardCredentialProvider.dll built successfully!
dir bin\SmartCardCredentialProvider.dll | findstr SmartCardCredentialProvider.dll
exit /b 0
