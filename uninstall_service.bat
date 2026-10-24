@echo off
chcp 65001 >nul
title Windows 插卡解锁 - 卸载系统服务

net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [提示] 正在请求 Windows 管理员权限以卸载系统服务...
    powershell -Command "Start-Process cmd.exe -ArgumentList '/c \"\"%~dpnx0\"\"' -Verb RunAs"
    exit /b
)

cd /d "%~dp0"
echo ==============================================================
echo   Windows 插卡解锁服务 (RTS5169 + IC4442) - 卸载服务
echo ==============================================================
echo.

"%~dp0bin\smartcard_unlock.exe" uninstall-service

echo.
pause
