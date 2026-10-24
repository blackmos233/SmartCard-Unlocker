@echo off
chcp 65001 >nul
title Windows 插卡解锁 - 服务热更新

net session >nul 2>&1
if errorlevel 1 (
    powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b
)

cd /d "%~dp0"
echo ==============================================================
echo   Windows 插卡解锁服务 (RTS5169 + IC4442) - 服务热更新
echo ==============================================================
echo.

echo [1/3] 正在停止 SmartCardUnlock 后台服务...
net stop SmartCardUnlock >nul 2>&1
timeout /t 1 /nobreak >nul

echo [2/3] 正在替换核心二进制文件 (覆盖为最新修复版)...
copy /y "%~dp0rust-core\target\release\smartcard_unlock.exe" "%~dp0bin\smartcard_unlock.exe"
if errorlevel 1 (
    echo [错误] 替换文件失败！请确认是否有进程正在占用。
    pause
    exit /b
)

echo [3/3] 正在重新启动 SmartCardUnlock 服务...
net start SmartCardUnlock

echo.
echo ==============================================================
echo [OK] 服务更新成功！最新原生引擎已生效并保持后台运行！
echo ==============================================================
echo.
"%~dp0bin\smartcard_unlock.exe" status
echo.
pause
