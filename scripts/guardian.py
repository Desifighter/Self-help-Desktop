#!/usr/bin/env python3
"""
Supervision Desktop - Live Foreground Window & Process Guardian
Monitors active browser windows every 100ms.
Features:
1. Adult Website Interception: Minimizes & closes tabs viewing adult content.
2. Anti-Tor Neutralizer: Detects and kills any Brave/standalone Tor process (tor*.exe).
3. Tor Window Blocker: Minimizes and closes any Brave "Private Window with Tor".
"""

import sys
import os
import time
import ctypes
from ctypes import wintypes
from pathlib import Path

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

SW_MINIMIZE = 6
WM_CLOSE = 0x0010
VK_CONTROL = 0x11
KEYEVENTF_KEYUP = 0x0002

BASE_DIR = Path(__file__).resolve().parent.parent
ADULT_DOMAINS_FILE = BASE_DIR / "assets" / "blocklists" / "adult_domains.txt"
LOG_FILE = BASE_DIR / "supervision_guardian.log"

# Load adult domain list
adult_stems = set()
if ADULT_DOMAINS_FILE.exists():
    with open(ADULT_DOMAINS_FILE, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                clean = line.lower().removeprefix("www.").strip()
                if "." in clean:
                    stem = clean.split(".")[0]
                    if len(stem) > 2:
                        adult_stems.add(stem)

# Adult TLDs and Strong Keywords
ADULT_TLDS = [".xxx", ".porn", ".sex", ".adult", ".sexy", ".cam", ".tube", ".erotica"]
STRONG_KEYWORDS = [
    "pornhub", "xvideos", "xnxx", "xhamster", "xham", "redtube", "youporn", "tube8",
    "spankbang", "brazzers", "bangbros", "realitykings", "chaturbate", "livejasmin",
    "stripchat", "onlyfans", "fansly", "manyvids", "rule34", "e-hentai", "nhentai",
    "porntrex", "tnaflix", "drtuber", "beeg", "txxx", "eporner", "heavy-r",
    "hqporner", "daftsex", "motherless", "empflix", "slutload", "nuvid", "tblop",
    "cam4", "camsoda", "bongacams", "flirt4free", "myfreecams", "hentaihaven",
    "hanime", "hitomi", "gelbooru", "danbooru", "desibabe", "antvasna", "kamababa",
    "desikahani", "savita", "bhabhi", "malluporn", "tamilporn", "teluguporn",
    "banglaporn", "hindisex", "indianporn", "indiansex", "blowjob", "handjob",
    "creampie", "gangbang", "cuckold", "masturbat", "orgasm", "ejaculat", "cumshot",
    "upskirt", "javhd", "javlibrary", "javbus"
]

SAFE_OVERRIDES = [
    "visual studio", "github", "gitlab", "stackoverflow", "crates.io",
    "docs.rs", "teams", "outlook", "slack", "zoom", "google search",
    "wikipedia", "python", "rust", "code", "terminal"
]

def kill_tor_processes():
    """Scans and terminates any Tor client processes (e.g. Brave Tor tor-*.exe)"""
    PROCESS_TERMINATE = 0x0001
    SNAPSHOT_PROCESS = 0x00000002

    class PROCESSENTRY32(ctypes.Structure):
        _fields_ = [
            ('dwSize', wintypes.DWORD),
            ('cntUsage', wintypes.DWORD),
            ('th32ProcessID', wintypes.DWORD),
            ('th32DefaultHeapID', ctypes.c_size_t),
            ('th32ModuleID', wintypes.DWORD),
            ('cntThreads', wintypes.DWORD),
            ('th32ParentProcessID', wintypes.DWORD),
            ('pcPriClassBase', ctypes.c_long),
            ('dwFlags', wintypes.DWORD),
            ('szExeFile', ctypes.c_char * 260)
        ]

    h_snap = kernel32.CreateToolhelp32Snapshot(SNAPSHOT_PROCESS, 0)
    if h_snap != -1:
        pe = PROCESSENTRY32()
        pe.dwSize = ctypes.sizeof(PROCESSENTRY32)
        if kernel32.Process32First(h_snap, ctypes.byref(pe)):
            while True:
                exe_name = pe.szExeFile.decode('latin1', errors='ignore').lower()
                if exe_name.startswith("tor-") or exe_name == "tor.exe" or "tor-browser" in exe_name:
                    h_proc = kernel32.OpenProcess(PROCESS_TERMINATE, False, pe.th32ProcessID)
                    if h_proc:
                        kernel32.TerminateProcess(h_proc, 1)
                        kernel32.CloseHandle(h_proc)
                        with open(LOG_FILE, "a", encoding="utf-8") as f:
                            f.write(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] TERMINATED TOR PROCESS: {exe_name} (PID: {pe.th32ProcessID})\n")
                if not kernel32.Process32Next(h_snap, ctypes.byref(pe)):
                    break
        kernel32.CloseHandle(h_snap)

def is_adult_title(title: str) -> tuple[bool, str]:
    t = title.lower()
    if not t or len(t) < 3:
        return False, ""

    # Anti-Tor Check: Block Brave Private Window with Tor
    if "private window with tor" in t or "(tor)" in t:
        return True, "Brave Tor Window (Bypass Attempt Blocked)"

    # Never interfere with safe work/dev software
    for safe in SAFE_OVERRIDES:
        if safe in t:
            return False, ""

    for kw in STRONG_KEYWORDS:
        if kw in t:
            return True, f"Keyword '{kw}'"

    for tld in ADULT_TLDS:
        if tld in t:
            return True, f"Adult TLD '{tld}'"

    for stem in adult_stems:
        if len(stem) >= 4 and stem in t:
            return True, f"Domain stem '{stem}'"

    return False, ""

def get_process_name(pid: int) -> str:
    PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
    h_proc = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
    if not h_proc:
        return ""
    buf = ctypes.create_unicode_buffer(1024)
    size = wintypes.DWORD(len(buf))
    res = kernel32.QueryFullProcessImageNameW(h_proc, 0, buf, ctypes.byref(size))
    kernel32.CloseHandle(h_proc)
    if res:
        return Path(buf.value).name.lower()
    return ""

def trigger_close_tab(hwnd):
    # Send Ctrl+W directly to close the active tab
    user32.keybd_event(VK_CONTROL, 0, 0, 0)
    user32.keybd_event(ord('W'), 0, 0, 0)
    user32.keybd_event(ord('W'), 0, KEYEVENTF_KEYUP, 0)
    user32.keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0)

def main():
    print("=" * 60)
    print("   Supervision Desktop — Real-Time Active Guardian Running   ")
    print("=" * 60)
    print("Active protections:")
    print("  • Adult domain & keyword interception")
    print("  • Anti-Tor process killer (tor*.exe)")
    print("  • Brave Tor window closer")
    print("Monitoring every 100ms...\n")

    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] Guardian started with Anti-Tor protection\n")

    last_evicted_time = 0
    last_evicted_title = ""

    while True:
        try:
            # 1. Kill any active Tor background processes
            kill_tor_processes()

            # 2. Check active foreground window
            hwnd = user32.GetForegroundWindow()
            if hwnd:
                length = user32.GetWindowTextLengthW(hwnd)
                if length > 0:
                    buff = ctypes.create_unicode_buffer(length + 1)
                    user32.GetWindowTextW(hwnd, buff, length + 1)
                    title = buff.value

                    pid = wintypes.DWORD()
                    user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
                    pname = get_process_name(pid.value)

                    is_browser = any(b in pname for b in ["brave", "chrome", "msedge", "firefox", "opera", "vivaldi"])

                    if is_browser:
                        is_adult, reason = is_adult_title(title)
                        if is_adult:
                            now = time.time()
                            if title != last_evicted_title or (now - last_evicted_time) > 1.5:
                                last_evicted_title = title
                                last_evicted_time = now

                                log_entry = f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] BLOCKED: {pname} | Title: '{title}' | Reason: {reason}"
                                print(f"\n⛔ {log_entry}")
                                with open(LOG_FILE, "a", encoding="utf-8") as f:
                                    f.write(log_entry + "\n")

                                # Stage 1: Minimize immediately
                                user32.ShowWindow(hwnd, SW_MINIMIZE)

                                # Stage 2: Send Ctrl+W to close prohibited tab
                                trigger_close_tab(hwnd)

                                # Stage 3: Send WM_CLOSE if whole window is Tor
                                time.sleep(0.05)
                                user32.PostMessageW(hwnd, WM_CLOSE, 0, 0)

        except Exception:
            pass

        time.sleep(0.1)

if __name__ == "__main__":
    main()
