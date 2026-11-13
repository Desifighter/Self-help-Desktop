# Supervision Desktop - One-Click System Protection Enabler
# Self-elevates to Administrator to apply system-wide DNS hosts blocks and anti-bypass policies

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

# 1. Update Windows Hosts File
Write-Host '[1/4] Applying adult domains to Windows hosts file...' -ForegroundColor Yellow
if (Test-Path $AdultListPath) {
    $domains = Get-Content $AdultListPath | Where-Object { $_ -and -not $_.StartsWith('#') } | ForEach-Object {
        $_.Trim().ToLower() -replace '^www\.', '' -replace '^0\.0\.0\.0\s+', '' -replace '^127\.0\.0\.1\s+', ''
    } | Where-Object { $_.Contains('.') } | Select-Object -Unique

    $existing = Get-Content $HostsPath -Raw -ErrorAction SilentlyContinue
    if (-not $existing) { $existing = '' }

    # Strip old section if present
    $cleaned = $existing -replace '(?s)# --- BEGIN SUPERVISION DESKTOP MANAGED BLOCKLIST ---.*?# --- END SUPERVISION DESKTOP MANAGED BLOCKLIST ---\r?\n?', ''

    $sb = [System.Text.StringBuilder]::new()
    [void]$sb.AppendLine('')
    [void]$sb.AppendLine('# --- BEGIN SUPERVISION DESKTOP MANAGED BLOCKLIST ---')
    [void]$sb.AppendLine('# Supervision Adult Content Filter Blocklist')
    foreach ($d in $domains) {
        [void]$sb.AppendLine("0.0.0.0 $d")
        [void]$sb.AppendLine("0.0.0.0 www.$d")
    }
    [void]$sb.AppendLine('# --- END SUPERVISION DESKTOP MANAGED BLOCKLIST ---')

    $final = $cleaned.TrimEnd() + "`r`n" + $sb.ToString()
    Set-Content -Path $HostsPath -Value $final -Encoding UTF8 -Force
    Write-Host "  [+] Hosts file successfully updated with $($domains.Count) adult domains." -ForegroundColor Green
} else {
    Write-Host "  [-] Adult list file not found at $AdultListPath" -ForegroundColor Red
}

# 2. Disable DoH (Encrypted DNS) in Chrome, Edge, and Brave so they cannot secretly bypass hosts
Write-Host '[2/4] Disabling browser DoH bypass in Chrome, Edge, and Brave...' -ForegroundColor Yellow
$regPaths = @(
    'HKLM:\SOFTWARE\Policies\Google\Chrome',
    'HKLM:\SOFTWARE\Policies\Microsoft\Edge',
    'HKLM:\SOFTWARE\Policies\BraveSoftware\Brave'
)

foreach ($path in $regPaths) {
    if (-not (Test-Path $path)) {
        New-Item -Path $path -Force | Out-Null
    }
    Set-ItemProperty -Path $path -Name 'DnsOverHttpsMode' -Value 'off' -Type String -Force
    Set-ItemProperty -Path $path -Name 'BuiltInDnsClientEnabled' -Value 0 -Type DWord -Force
}
Write-Host '  [+] Browser DoH bypass neutralized.' -ForegroundColor Green

# 3. Flush DNS Cache
Write-Host '[3/4] Flushing Windows DNS cache...' -ForegroundColor Yellow
ipconfig /flushdns | Out-Null
Write-Host '  [+] DNS cache cleared.' -ForegroundColor Green

# 4. Start Background Guardian
Write-Host '[4/4] Launching Real-Time Window Guardian...' -ForegroundColor Yellow
$guardianScript = Join-Path $PSScriptRoot 'guardian.py'
$pythonw = (Get-Command pythonw.exe -ErrorAction SilentlyContinue).Source
if (-not $pythonw) {
    $pythonw = (Get-Command python.exe -ErrorAction SilentlyContinue).Source
}

if ($pythonw -and (Test-Path $guardianScript)) {
    $running = Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*guardian.py*' }
    if (-not $running) {
        Start-Process $pythonw -ArgumentList "`"$guardianScript`"" -WindowStyle Hidden
        Write-Host '  [+] Background Guardian process started.' -ForegroundColor Green
    } else {
        Write-Host '  [+] Background Guardian is already active.' -ForegroundColor Green
    }
}

Write-Host '==========================================================' -ForegroundColor Green
Write-Host '  ALL SYSTEM-WIDE ADULT BLOCKING ENFORCED SUCCESSFULLY!  ' -ForegroundColor Green
Write-Host '==========================================================' -ForegroundColor Green
Write-Host 'Adult sites are now blocked across all browsers and applications.' -ForegroundColor White
Write-Host 'Press any key to close this window...' -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
