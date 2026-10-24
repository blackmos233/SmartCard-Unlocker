@echo off
setlocal
cd /d "%~dp0"
echo ==============================================================
echo   Update Windows Unlock Password
echo ==============================================================
echo.
set /p NEWPASS=Please enter your new Windows password: 
if "%NEWPASS%"=="" (
    echo [ERROR] Password cannot be empty!
    pause
    exit /b 1
)
"%~dp0bin\smartcard_unlock.exe" set-pin "%NEWPASS%"
echo.
echo [OK] Password updated successfully in secure vault!
pause
