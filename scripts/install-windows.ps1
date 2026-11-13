# Supervision Desktop - Windows Service Installation Script
# Must be executed in an elevated Administrator PowerShell session

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   Supervision Desktop - Windows Installation & Setup     " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Check Administrator Privileges
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "This script requires Administrator privileges. Please right-click and run PowerShell as Administrator."
    exit 1
}

$InstallDir = "$env:ProgramFiles\Supervision"
$DataDir = "$env:ProgramData\Supervision"

Write-Host "[1/5] Creating application directories..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

# 2. Build or copy binaries
$TargetDir = "$PSScriptRoot\..\target\release"
if (Test-Path "$TargetDir\supervision-daemon.exe") {
    Write-Host "[2/5] Deploying built binaries to $InstallDir..." -ForegroundColor Yellow
    Copy-Item "$TargetDir\supervision-daemon.exe" -Destination "$InstallDir\supervision-daemon.exe" -Force
    Copy-Item "$TargetDir\supervision-cli.exe" -Destination "$InstallDir\supervision-cli.exe" -Force
    Copy-Item "$TargetDir\supervision-overlay.exe" -Destination "$InstallDir\supervision-overlay.exe" -Force
} else {
    Write-Host "[2/5] Release binaries not found in target/release, compiling..." -ForegroundColor Yellow
    cargo build --release --bin supervision-daemon --bin supervision-cli --bin supervision-overlay
    Copy-Item "$TargetDir\supervision-daemon.exe" -Destination "$InstallDir\supervision-daemon.exe" -Force
    Copy-Item "$TargetDir\supervision-cli.exe" -Destination "$InstallDir\supervision-cli.exe" -Force
    Copy-Item "$TargetDir\supervision-overlay.exe" -Destination "$InstallDir\supervision-overlay.exe" -Force
}

# 3. Add to System PATH
$machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
if ($machinePath -notlike "*$InstallDir*") {
    Write-Host "[3/5] Adding Supervision to System PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("Path", "$machinePath;$InstallDir", "Machine")
}

# 4. Register Windows Service
Write-Host "[4/5] Registering Windows Service 'SupervisionService'..." -ForegroundColor Yellow
$serviceExists = Get-Service -Name "SupervisionService" -ErrorAction SilentlyContinue
if ($serviceExists) {
    Write-Host "Stopping and updating existing service..." -ForegroundColor Gray
    Stop-Service -Name "SupervisionService" -Force -ErrorAction SilentlyContinue
    sc.exe delete SupervisionService | Out-Null
    Start-Sleep -Seconds 1
}

$daemonPath = "$InstallDir\supervision-daemon.exe service"
sc.exe create SupervisionService binPath= "`"$daemonPath`"" start= auto DisplayName= "Supervision Desktop Service"
sc.exe description SupervisionService "Privacy-First Digital Self-Control and Network Filtering Background Service"

# Configure failure recovery: restart automatically on error
sc.exe failure SupervisionService reset= 86400 actions= restart/1000/restart/5000/restart/10000

# 5. Start Service
Write-Host "[5/5] Starting Supervision Service..." -ForegroundColor Yellow
Start-Service -Name "SupervisionService"

Write-Host "`n✓ Supervision Desktop Service successfully installed and running!" -ForegroundColor Green
Write-Host "Run 'supervision-cli status' to verify daemon health and blocking stats." -ForegroundColor Cyan
