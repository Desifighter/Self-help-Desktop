@echo off
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process powershell.exe -Verb RunAs -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"\"%~dp0enable_system_protection.ps1\"\"'"
