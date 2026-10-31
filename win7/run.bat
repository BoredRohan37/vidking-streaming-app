@echo off
setlocal
cd /d "%~dp0"
title MovieBox-TUI
mode con: cols=120 lines=36
moviebox-tui.exe %*
if errorlevel 1 (
  echo.
  echo MovieBox-TUI exited with an error. Press any key to close.
  pause >nul
)
endlocal
