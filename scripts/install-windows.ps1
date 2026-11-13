# Supervision Desktop - Windows Service Installation & Hardening Script
# Must be executed in an elevated Administrator PowerShell session

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   Supervision Desktop - Windows Hardened Installation    " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Check Administrator Privileges
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "This script requires Administrator privileges. Please right-click and run PowerShell as Administrator."
    exit 1
}

$InstallDir = "$env:ProgramFiles\Supervision"
$DataDir = "$env:ProgramData\Supervision"

Write-Host "[1/6] Creating application directories..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

# 2. Build or copy binaries
$TargetDir = "$PSScriptRoot\..\target\release"
if (Test-Path "$TargetDir\supervision-daemon.exe") {
    Write-Host "[2/6] Deploying built binaries to $InstallDir..." -ForegroundColor Yellow
    Copy-Item "$TargetDir\supervision-daemon.exe" -Destination "$InstallDir\supervision-daemon.exe" -Force
    Copy-Item "$TargetDir\supervision-cli.exe" -Destination "$InstallDir\supervision-cli.exe" -Force
    Copy-Item "$TargetDir\supervision-overlay.exe" -Destination "$InstallDir\supervision-overlay.exe" -Force
} else {
    Write-Host "[2/6] Release binaries not found in target/release, compiling..." -ForegroundColor Yellow
    cargo build --release --bin supervision-daemon --bin supervision-cli --bin supervision-overlay
    Copy-Item "$TargetDir\supervision-daemon.exe" -Destination "$InstallDir\supervision-daemon.exe" -Force
    Copy-Item "$TargetDir\supervision-cli.exe" -Destination "$InstallDir\supervision-cli.exe" -Force
    Copy-Item "$TargetDir\supervision-overlay.exe" -Destination "$InstallDir\supervision-overlay.exe" -Force
}

# 3. Add to System PATH
$machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
if ($machinePath -notlike "*$InstallDir*") {
    Write-Host "[3/6] Adding Supervision to System PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("Path", "$machinePath;$InstallDir", "Machine")
}

# 4. Anti-Tamper ACL Protection (Lock files so standard users cannot delete/tamper)
Write-Host "[4/6] Applying Anti-Tamper ACL filesystem permissions..." -ForegroundColor Yellow
# Grant SYSTEM full control, Administrators read-and-execute, standard Users read-only
icacls.exe "$InstallDir" /inheritance:r /grant:r "SYSTEM:(OI)(CI)F" "Administrators:(OI)(CI)RX" "Users:(OI)(CI)RX" | Out-Null
icacls.exe "$DataDir" /inheritance:r /grant:r "SYSTEM:(OI)(CI)F" "Administrators:(OI)(CI)M" "Users:(OI)(CI)R" | Out-Null

# 5. Register Windows Service
Write-Host "[5/6] Registering Windows Service 'SupervisionService'..." -ForegroundColor Yellow
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

# Configure failure recovery: watchdog automatically restarts within 1 second if killed
sc.exe failure SupervisionService reset= 86400 actions= restart/1000/restart/2000/restart/5000

# 6. Start Service
Write-Host "[6/6] Starting Supervision Service..." -ForegroundColor Yellow
Start-Service -Name "SupervisionService"

Write-Host "`n✓ Supervision Desktop successfully installed and locked!" -ForegroundColor Green
Write-Host "✓ Anti-tamper watchdog and ACL protections active." -ForegroundColor Green
Write-Host "✓ Run 'supervision-cli status' to verify daemon health and blocking stats." -ForegroundColor Cyan
