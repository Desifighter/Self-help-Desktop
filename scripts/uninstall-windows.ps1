# Supervision Desktop - Windows Uninstallation Script
# Must be executed in an elevated Administrator PowerShell session

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "     Supervision Desktop - Protected Uninstallation       " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Check Administrator Privileges
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "Uninstallation requires Administrator privileges. Please run PowerShell as Administrator."
    exit 1
}

# 2. Check Anti-Tamper Lock & Cooling-Off Barrier via CLI
$cliPath = "$env:ProgramFiles\Supervision\supervision-cli.exe"
if (Test-Path $cliPath) {
    Write-Host "[1/4] Verifying disarm status with background daemon..." -ForegroundColor Yellow
    $statusOutput = & $cliPath status 2>&1 | Out-String

    if ($statusOutput -like "*LOCKED*") {
        Write-Host "`n⛔ UNINSTALL REJECTED: Anti-Tamper Protection Plan is Active!" -ForegroundColor Red
        Write-Host "Supervision is locked to prevent impulsive removal." -ForegroundColor Yellow
        Write-Host "To disarm the system, you must start the 15-minute cooling-off countdown:" -ForegroundColor White
        Write-Host "  > supervision-cli request-unlock" -ForegroundColor Cyan
        Write-Host "Once the 15-minute countdown finishes, run this uninstall script again.`n" -ForegroundColor White
        exit 1
    }
}

Write-Host "[2/4] Stopping and removing Supervision Service..." -ForegroundColor Yellow
Stop-Service -Name "SupervisionService" -Force -ErrorAction SilentlyContinue
sc.exe delete SupervisionService | Out-Null

Write-Host "[3/4] Restoring system hosts table..." -ForegroundColor Yellow
$hostsPath = "$env:SystemRoot\System32\drivers\etc\hosts"
if (Test-Path $hostsPath) {
    $content = Get-Content $hostsPath -Raw
    $cleaned = $content -replace "(?s)# --- BEGIN SUPERVISION DESKTOP MANAGED BLOCKLIST ---.*?# --- END SUPERVISION DESKTOP MANAGED BLOCKLIST ---\r?\n?", ""
    Set-Content $hostsPath -Value $cleaned.TrimEnd()
}

Write-Host "[4/4] Removing program files..." -ForegroundColor Yellow
$InstallDir = "$env:ProgramFiles\Supervision"
if (Test-Path $InstallDir) {
    # Remove ACL lock to permit deletion
    icacls.exe "$InstallDir" /grant "Administrators:F" /t | Out-Null
    Remove-Item -Recurse -Force $InstallDir -ErrorAction SilentlyContinue
}

Write-Host "`n✓ Supervision Desktop has been cleanly and safely removed." -ForegroundColor Green
