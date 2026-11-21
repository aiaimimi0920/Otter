@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Import-LoomConnection.ps1" -Executable "%~dp0..\payload\otter-native.exe"
if errorlevel 1 pause
