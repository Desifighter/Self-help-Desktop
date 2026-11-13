# Supervision Desktop — Architecture & Technical Specifications

Supervision Desktop is a cross-platform, local-first digital self-control, adult content blocking, and distraction intervention system for Windows, macOS, and Linux.

---

## 1. System Topology & Threat Model

```
+--------------------------------------------------------------------------+
|                        SUPERVISION DESKTOP SUITE                         |
+--------------------------------------------------------------------------+
                                     |
    +--------------------------------+--------------------------------+
    |                                                                 |
+---v----------------------------+       +----------------------------v----+
| Privileged Service / Daemon    |       | Unprivileged Client UI / CLI    |
| (SYSTEM / root)                |       | (Standard User Session)         |
| - supervision-daemon           |       | - supervision-desktop (Tauri v2)|
| - supervision-overlay          |       | - supervision-cli               |
+--------------------------------+       +---------------------------------+
    |                                        |
    |                                        |
    +------------ Local IPC (127.0.0.1:49215)+
                  Named Pipe / Domain Socket
```

### Threat Model & Bypass Mitigations
1. **Direct Process Kill**: Daemon runs as `NT AUTHORITY\SYSTEM` (Windows) or `root` (Linux/macOS). Standard user sessions cannot issue `TerminateProcess` or `SIGKILL` against the service.
2. **Impulsive Disarming**: Modifying or stopping filters initiates a 15-minute (900s) cooling-off delay barrier or requires an immutable emergency recovery key.
3. **Encrypted DNS (DoH / DoT) Bypass**: Browsers attempting to silently resolve prohibited domains via Cloudflare (1.1.1.1) or Google (8.8.8.8) over DoH (port 443/853) are neutralized via null-route IP blocklists and hosts synchronization.
4. **Subdomain Hopping**: Domain resolution normalizes subdomains (e.g. `cdn.sub.domain.xxx` is recursively checked against parent domain blocks).

---

## 2. Multi-Tier Domain Intelligence Pipeline

```
Incoming DNS Query (UDP 53)
           │
           ▼
[Tier 0: Fast In-Memory Cache] ──(Hit: Confirmed Safe/Blocked)──► Return Instant Response
           │ Miss
           ▼
[Tier 1: Safe Allowlist] ────────(Hit: Essential Tool)──────────► Forward to Clean Upstream DNS
           │ Miss
           ▼
[Tier 2: Maintained Blocklists] ─(Hit: Adult/Social/Gambling)──► Return 0.0.0.0 / NXDOMAIN
           │ Miss
           ▼
[Tier 3: On-Device Heuristics] ──(Confidence >= 0.85)──────────► Return 0.0.0.0 / NXDOMAIN
           │ Miss
           ▼
[Tier 4: Upstream Privacy Check] (Cloudflare Families 1.1.1.3)──► Return 0.0.0.0 if Blocked
           │ Clean
           ▼
Forward to Clean Upstream Resolver (1.1.1.1 / 8.8.8.8)
```

### Heuristic Token Analyzer
- **Adult TLDs**: `.xxx`, `.porn`, `.sex`, `.adult`, `.sexy`, `.cam`, `.tube`, `.erotica`.
- **Compound Rules**: Regional & Desi adult keywords (`bhabhi`, `antvasna`, `desikahani`, `kamababa`, etc.) and babe compounds (`sexybabe`, `hotbabe`, `cambabes`).
- **Safe Overrides**: Essential platforms (Google, Microsoft Teams, Outlook, GitHub, AWS, dev package registries) are immutably excluded from heuristic flagging.

---

## 3. Active Window & Process Eviction Engine

- **Windows**:
  - `GetForegroundWindow()` retrieves active window handle.
  - `GetWindowThreadProcessId()` retrieves owning PID.
  - `QueryFullProcessImageNameW()` resolves executable image path.
  - Multi-stage eviction:
    1. `ShowWindow(hwnd, SW_MINIMIZE)` and `PostMessageW(hwnd, WM_CLOSE, 0, 0)` for clean exit.
    2. Forceful fallback: `OpenProcess(PROCESS_TERMINATE)` + `TerminateProcess()`.
    3. Triggers fullscreen topmost `supervision-overlay`.
- **Linux**:
  - Inspects `/proc` and active window focus via X11 / Wayland protocols.
  - Multi-stage eviction: `SIGTERM` followed by `SIGKILL`.
- **macOS**:
  - Inspects frontmost app bundle ID via `NSWorkspace` / `System Events`.
  - Evicts via `SIGTERM` followed by `SIGKILL`.

---

## 4. Zero-Interference Whitelist Guarantee

Supervision Desktop strictly protects engineering, workplace, and system software:
- **Developer Toolchains**: `cargo`, `rustc`, `git`, `docker`, `node`, `python`, `powershell`, `bash`, `alacritty`, `wezterm`, `WindowsTerminal`.
- **IDEs**: VS Code (`code.exe`), JetBrains (`idea64.exe`, `pycharm64.exe`, `clion64.exe`, `rustrover64.exe`, `webstorm64.exe`), Sublime Text, Neovim.
- **Enterprise Workplace**: Microsoft Teams, Outlook, Slack, Zoom, Webex.
- **Critical Infrastructure**: Banking, tax portals, IRCTC/railway systems, OS update services.
