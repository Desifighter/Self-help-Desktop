# Supervision Desktop (Windows, macOS, Linux)
## Privacy-First Digital Self-Control, Pornography Blocking, Network Filtering & App Restriction Suite

[![Language](https://img.shields.io/badge/Language-Rust%20%7C%20TypeScript-orange.svg)](https://www.rust-lang.org/)
[![UI](https://img.shields.io/badge/UI-Tauri%20v2-blue.svg)](https://tauri.app/)
[![License](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-green.svg)](LICENSE)
[![Cross-Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)](README.md)

Supervision Desktop is the privacy-first, on-device companion for **Project Supervision** ([Android version](https://github.com/Desifighter/selfhelp)). Built with high-performance Rust, it provides system-wide network filtering, pornography blocking, and application restriction without telemetry, tracking, or cloud dependencies.

---

## Key Features

### 1. Local DNS Filtering Engine (System-Wide & Browser-Independent)
- **Local DNS Proxy:** Binds to `127.0.0.1:53` (UDP) to capture domain queries before browsers or apps reach the network.
- **Zero-Latency In-Memory Filtering:** Sub-millisecond NXDOMAIN or `0.0.0.0` responses for adult, gambling, and social media sites.
- **Multi-Tier Classification Engine:**
  1. Ultra-fast in-memory cache for repeated lookups.
  2. Maintained bundled blocklist (`assets/blocklists/adult_domains.txt`).
  3. On-device heuristics detecting adult TLDs (`.xxx`, `.porn`, `.adult`, `.cam`, etc.) and compound keywords.
  4. Privacy-preserving upstream queries against Cloudflare Families (`1.1.1.3`) and CleanBrowsing (`185.228.168.168`).
- **Anti-Bypass (DoH / Encrypted DNS Leak Prevention):**
  - Blocks or routes outbound public DoH resolver IPs (Cloudflare `1.1.1.1`, Google `8.8.8.8`, Quad9 `9.9.9.9`) on port 443/853 so browsers cannot secretly bypass local rules.
  - Automatic system hosts synchronization (`C:\Windows\System32\drivers\etc\hosts` / `/etc/hosts`) as a secondary enforcement layer.

### 2. Active Window & Application Restriction Engine
- **Process & Window Monitor:**
  - **Windows:** Win32 APIs (`GetForegroundWindow`, `GetWindowThreadProcessId`, `QueryFullProcessImageNameW`).
  - **Linux:** `/proc` inspection and X11/Wayland active window queries.
  - **macOS:** `NSWorkspace` / Accessibility inspection.
- **Multi-Stage Eviction:**
  - Stage 1: Graceful closure request (`WM_CLOSE` / `SIGTERM`).
  - Stage 2: Immediate window minimize (`ShowWindow(SW_MINIMIZE)`).
  - Stage 3: Forceful termination (`TerminateProcess` / `SIGKILL`) if persistent.
- **Layered Restriction Overlay:**
  - Instant fullscreen topmost alert screen (`supervision-overlay`) alerting the user when restricted content or applications are triggered.

### 3. System-Level Persistence & Anti-Tamper
- **Privileged Background Service:**
  - **Windows:** Runs as a native Windows Service under SCM (`NT AUTHORITY\SYSTEM`).
  - **Linux:** Native `systemd` system service running as root.
  - **macOS:** `launchd` system LaunchDaemon running as root.
- **Self-Healing Watchdog:**
  - Automatic restart on failure within 1 second.
- **Friction Barrier (Cooling-Off Period):**
  - Disarming restrictions requires starting a 15-minute (900-second) cooling-off delay to eliminate impulsive bypasses.

### 4. Zero-Interference Whitelist Guarantee
- **Engineering & Developer Tools:** Cargo, Rustc, Git, Docker, Node.js, Python, VS Code, JetBrains IDEs, Windows Terminal, PowerShell, Bash.
- **Workplace Platforms:** Microsoft Teams, Outlook, Slack, Zoom, Webex, Office 365, Notion, Jira.
- **Critical Infrastructure:** Online banking, IRCTC, tax services, OS update daemons.

---

## Project Structure

```
Self-Help-Desktop/
├── assets/
│   └── blocklists/
│       ├── adult_domains.txt         # Bundled adult domain blocklist
│       ├── doh_ips.txt               # Public DoH resolver IPs for leak prevention
│       └── essential_allowlist.txt   # Essential workplace & developer whitelist
├── crates/
│   ├── supervision-core/             # Core library (DNS, heuristics, monitor, IPC)
│   ├── supervision-daemon/           # Privileged background service daemon
│   ├── supervision-cli/              # Admin CLI tool for testing and management
│   ├── supervision-overlay/          # Fullscreen topmost restriction alert window
│   └── supervision-desktop/          # Tauri v2 desktop dashboard app
│       ├── src-tauri/                # Tauri backend and IPC bridge
│       └── ui/                       # Modern dark-mode responsive dashboard
├── scripts/
│   ├── install-windows.ps1           # Windows Service installation script
│   ├── install-linux.sh              # Linux systemd installation script
│   └── install-macos.sh              # macOS launchd installation script
├── ARCHITECTURE.md                   # Detailed technical specifications
├── Cargo.toml                        # Workspace configuration
└── README.md
```

---

## Installation & Setup

### Prerequisites
- [Rust](https://rustup.rs/) (version 1.75+ or stable)
- Node.js (v18+) for building the Tauri UI

### 1. Build from Source
```bash
# Build the daemon, CLI, and overlay
cargo build --release

# Run unit tests
cargo test
```

### 2. Windows Service Installation (Run as Administrator)
Open PowerShell as Administrator:
```powershell
.\scripts\install-windows.ps1
```
This registers and starts `SupervisionService` running as `NT AUTHORITY\SYSTEM` with automatic failure restart.

### 3. Linux Installation (systemd)
```bash
sudo bash ./scripts/install-linux.sh
```

### 4. macOS Installation (LaunchDaemon)
```bash
sudo bash ./scripts/install-macos.sh
```

---

## Using the Administrative CLI (`supervision-cli`)

The CLI communicates with the background daemon over the secure local IPC channel (`127.0.0.1:49215`):

```bash
# Check service health and live statistics
supervision-cli status

# Test any domain against blocklists, heuristics, and upstream verification
supervision-cli test-domain pornhub.com
supervision-cli test-domain github.com

# Add custom blocked domains
supervision-cli block-domain omegle.com

# Remove custom blocked domain
supervision-cli unblock-domain omegle.com

# Add restricted application
supervision-cli block-app discord.exe

# Start cooling-off disarm countdown (15 minutes)
supervision-cli request-unlock

# View recent intervention events
supervision-cli logs
```

---

## Running the Desktop Dashboard (`supervision-desktop`)

To launch the Tauri v2 dashboard:
```bash
cd crates/supervision-desktop
cargo tauri dev
```
Or directly inspect the pre-compiled UI in `crates/supervision-desktop/ui/dist/index.html`.

---

## Companion Applications
- **Android Companion:** [Desifighter/selfhelp](https://github.com/Desifighter/selfhelp) — On-device VPN-based adult blocking and digital self-control for Android 12-16.

---

## License
Licensed under either of Apache License, Version 2.0 or MIT license at your option.
