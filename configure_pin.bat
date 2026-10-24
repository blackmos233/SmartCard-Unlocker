@echo off
chcp 65001 >nul
title Windows Hello PIN 码配置向导
cd /d "%~dp0"
"%~dp0bin\smartcard_unlock.exe" set-pin
pause
