@echo off
title SmartCard Unlock - PIN Enrollment
cd /d "%~dp0"
"%~dp0bin\smartcard_unlock.exe" set-pin
pause
