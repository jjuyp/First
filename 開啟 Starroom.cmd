@echo off
chcp 65001 >nul
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\start-starroom.ps1"
if errorlevel 1 (
  echo.
  echo Starroom 啟動失敗，請保留此視窗中的錯誤訊息。
  pause
)
