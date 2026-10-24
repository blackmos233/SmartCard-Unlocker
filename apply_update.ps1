# apply_update.ps1
$ErrorActionPreference = "Continue"
Write-Host "=== SmartCardUnlock 服务热更新 ==="
Write-Host "[1/3] 正在停止 SmartCardUnlock 服务..."
Stop-Service -Name "SmartCardUnlock" -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 800

Write-Host "[2/3] 正在覆盖二进制文件..."
Copy-Item -Path "D:\work\SmartCard\rust-core\target\release\smartcard_unlock.exe" -Destination "D:\work\SmartCard\bin\smartcard_unlock.exe" -Force

Write-Host "[3/3] 正在重新启动服务..."
Start-Service -Name "SmartCardUnlock"

Write-Host "更新成功！"
Start-Sleep -Seconds 1
