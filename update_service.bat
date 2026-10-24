@echo off
chcp 65001 >nul
title Windows 插卡解锁 - 服务热更新

net session >nul 2>&1
if %errorLevel% neq 0 (
    powershell -Command "Start-Process cmd.exe -ArgumentList '/c \"\"%~dpnx0\"\"' -Verb RunAs"
    exit /b
)

cd /d "%~dp0"
echo ==============================================================
echo   Windows 插卡解锁服务 (RTS5169 + IC4442) - 热更新
echo ==============================================================
echo.

echo [1/3] 正在停止 SmartCardUnlock 服务...
net stop SmartCardUnlock >nul 2>&1

echo [2/3] 正在替换核心二进制文件...
copy /y "%~dp0rust-core\target\release\smartcard_unlock.exe" "%~dp0bin\smartcard_unlock.exe"

echo [3/3] 正在启动最新版服务...
net start SmartCardUnlock

echo.
echo ==============================================================
echo [OK] 服务热更新成功！最新原生引擎已上线！
echo ==============================================================
echo.
"%~dp0bin\smartcard_unlock.exe" status
echo.
pause
