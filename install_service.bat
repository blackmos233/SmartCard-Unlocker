@echo off
chcp 65001 >nul
title Windows 插卡解锁 - 一键安装系统服务

net session >nul 2>&1
if errorlevel 1 (
    powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b
)

cd /d "%~dp0"
echo ==============================================================
echo   Windows 插卡解锁服务 (RTS5169 + IC4442) - 一键安装
echo ==============================================================
echo.

"%~dp0bin\smartcard_unlock.exe" install-service

echo.
echo 当前服务与设备运行状态:
"%~dp0bin\smartcard_unlock.exe" status

echo.
echo ==============================================================
echo 安装完成！
echo 服务已设为开机自动启动并在后台默默守护。
echo 拔出卡片立即锁屏，插入卡片自动在锁屏界面注入 PIN 码解锁！
echo ==============================================================
pause
