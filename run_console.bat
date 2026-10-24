@echo off
chcp 65001 >nul
title Windows 插卡解锁 - 控制台交互菜单
cd /d "%~dp0"
"%~dp0bin\smartcard_unlock.exe"
pause
