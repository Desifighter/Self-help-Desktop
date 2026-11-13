@echo off
title Supervision Desktop - Enable System-Wide Protection
echo Requesting Administrator privileges...
powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process powershell.exe -Verb RunAs -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"\"%~dp0enable_system_protection.ps1\"\"'"
