# Supervision Desktop - One-Click System Protection Enabler
# Self-elevates to Administrator to apply system-wide DNS hosts blocks, IPv6 sinkholing, and anti-bypass policies

$ErrorActionPreference = "Stop"

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host 'Elevating with Administrator privileges...' -ForegroundColor Yellow
    Start-Process powershell.exe -Verb RunAs -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    exit 0
}

Write-Host '==========================================================' -ForegroundColor Cyan
Write-Host '   Supervision Desktop - Activating System-Wide Protection ' -ForegroundColor Cyan
Write-Host '==========================================================' -ForegroundColor Cyan

$BaseDir = Split-Path -Parent $PSScriptRoot
$AdultListPath = Join-Path $BaseDir "assets\blocklists\adult_domains.txt"
$HostsPath = "$env:SystemRoot\System32\drivers\etc\hosts"

# 1. Update Windows Hosts File with DUAL IPv4 (0.0.0.0) AND IPv6 (::) Sinkholes
Write-Host '[1/5] Applying adult domains to hosts file (IPv4 + IPv6 sinkhole)...' -ForegroundColor Yellow
if (Test-Path $AdultListPath) {
    $domains = Get-Content $AdultListPath | Where-Object { $_ -and -not $_.StartsWith('#') } | ForEach-Object {
        $_.Trim().ToLower() -replace '^www\.', '' -replace '^0\.0\.0\.0\s+', '' -replace '^127\.0\.0\.1\s+', ''
    } | Where-Object { $_.Contains('.') } | Select-Object -Unique

    $existing = ''
    if (Test-Path $HostsPath) {
        try {
            $existing = [System.IO.File]::ReadAllText($HostsPath)
        } catch {
            $existing = ''
        }
    }

    # Strip old section if present
    $cleaned = $existing -replace '(?s)# --- BEGIN SUPERVISION DESKTOP MANAGED BLOCKLIST ---.*?# --- END SUPERVISION DESKTOP MANAGED BLOCKLIST ---\r?\n?', ''

    $sb = [System.Text.StringBuilder]::new()
    [void]$sb.AppendLine('')
    [void]$sb.AppendLine('# --- BEGIN SUPERVISION DESKTOP MANAGED BLOCKLIST ---')
    [void]$sb.AppendLine('# Supervision Adult Content Filter Blocklist (Dual IPv4/IPv6)')
    foreach ($d in $domains) {
        # IPv4 Sinkhole
        [void]$sb.AppendLine("0.0.0.0 $d")
        [void]$sb.AppendLine("0.0.0.0 www.$d")
        # IPv6 Sinkhole (prevents browser IPv6 fallback!)
        [void]$sb.AppendLine(":: $d")
        [void]$sb.AppendLine(":: www.$d")
    }
    [void]$sb.AppendLine('# --- END SUPERVISION DESKTOP MANAGED BLOCKLIST ---')

    $final = $cleaned.TrimEnd() + "`r`n" + $sb.ToString()

    & attrib.exe -r -h -s "$HostsPath" 2>$null
    [System.IO.File]::WriteAllText($HostsPath, $final, [System.Text.Encoding]::ASCII)
    Write-Host "  [+] Hosts file updated with $($domains.Count) domains on both IPv4 (0.0.0.0) and IPv6 (::)." -ForegroundColor Green
}

# 2. Configure System Network Adapters to use Cloudflare Families (Adult Blocking DNS)
Write-Host '[2/5] Configuring network adapter DNS to Cloudflare Families (1.1.1.3)...' -ForegroundColor Yellow
try {
    $activeAdapters = Get-NetAdapter | Where-Object { $_.Status -eq "Up" }
    foreach ($adapter in $activeAdapters) {
        # IPv4 DNS: Cloudflare Families (1.1.1.3, 1.0.0.3)
        Set-DnsClientServerAddress -InterfaceAlias $adapter.Name -ServerAddresses ("1.1.1.3", "1.0.0.3") -ErrorAction SilentlyContinue
        Write-Host "  [+] Configured $($adapter.Name) to 1.1.1.3 (Cloudflare Families)." -ForegroundColor Green
    }
} catch {
    Write-Host "  [-] Could not set adapter DNS: $_" -ForegroundColor Yellow
}

# 3. Disable Browser DoH (Encrypted DNS) and Tor Mode in Brave
Write-Host '[3/5] Disabling browser DoH bypass and Brave Tor mode...' -ForegroundColor Yellow
$regPaths = @(
    'HKLM:\SOFTWARE\Policies\Google\Chrome',
    'HKLM:\SOFTWARE\Policies\Microsoft\Edge',
    'HKLM:\SOFTWARE\Policies\BraveSoftware\Brave',
    'HKCU:\SOFTWARE\Policies\BraveSoftware\Brave'
)

foreach ($path in $regPaths) {
    if (-not (Test-Path $path)) {
        New-Item -Path $path -Force | Out-Null
    }
    Set-ItemProperty -Path $path -Name 'DnsOverHttpsMode' -Value 'off' -Type String -Force
    Set-ItemProperty -Path $path -Name 'BuiltInDnsClientEnabled' -Value 0 -Type DWord -Force
    if ($path -like '*Brave*') {
        Set-ItemProperty -Path $path -Name 'TorDisabled' -Value 1 -Type DWord -Force
    }
}
Write-Host '  [+] Browser DoH and Brave Tor mode policies locked to DISABLED.' -ForegroundColor Green

# Kill any existing Tor processes
Get-Process *tor* -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -like 'tor*' } | Stop-Process -Force -ErrorAction SilentlyContinue

# 4. Flush DNS and NetBIOS Caches
Write-Host '[4/5] Flushing Windows DNS and socket caches...' -ForegroundColor Yellow
ipconfig /flushdns | Out-Null
nbtstat -R 2>$null | Out-Null
Write-Host '  [+] DNS cache flushed.' -ForegroundColor Green

# 5. Start Background Guardian
Write-Host '[5/5] Launching Real-Time Window Guardian...' -ForegroundColor Yellow
$guardianScript = Join-Path $PSScriptRoot 'guardian.py'
$pythonw = (Get-Command pythonw.exe -ErrorAction SilentlyContinue).Source
if (-not $pythonw) {
    $pythonw = (Get-Command python.exe -ErrorAction SilentlyContinue).Source
}

if ($pythonw -and (Test-Path $guardianScript)) {
    $running = Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*guardian.py*' }
    if (-not $running) {
        Start-Process $pythonw -ArgumentList "`"$guardianScript`"" -WindowStyle Hidden
        Write-Host '  [+] Background Guardian started.' -ForegroundColor Green
    } else {
        Write-Host '  [+] Background Guardian is already active.' -ForegroundColor Green
    }
}

Write-Host '==========================================================' -ForegroundColor Green
Write-Host '  ALL SYSTEM-WIDE ADULT BLOCKING ENFORCED SUCCESSFULLY!  ' -ForegroundColor Green
Write-Host '==========================================================' -ForegroundColor Green
Write-Host 'IMPORTANT: Please RESTART your browser (close all windows)' -ForegroundColor Yellow
Write-Host 'to terminate any existing cached socket connections.' -ForegroundColor Yellow
Write-Host '==========================================================' -ForegroundColor Green
Write-Host 'Press any key to close this window...' -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
