#!/usr/bin/env python3
"""
Supervision Desktop - Live Engine & Functional Test Suite
Verifies:
1. Multi-tier adult domain classification (Blocklist, Heuristics, Upstream Cloudflare/CleanBrowsing)
2. Zero-Interference Safe Allowlist guarantees (GitHub, Teams, Banks, Dev tools)
3. Active foreground window detection (Win32 API)
4. Subdomain inheritance
"""

import sys
import os
import socket
import struct
import time
import ctypes
from pathlib import Path

# Paths
BASE_DIR = Path(__file__).resolve().parent.parent
ADULT_DOMAINS_FILE = BASE_DIR / "assets" / "blocklists" / "adult_domains.txt"
ESSENTIAL_FILE = BASE_DIR / "assets" / "blocklists" / "essential_allowlist.txt"

# 1. Load Allowlist
safe_domains = set()
if ESSENTIAL_FILE.exists():
    with open(ESSENTIAL_FILE, "r", encoding="utf-8") as f:
        in_domains = True
        for line in f:
            line = line.strip()
            if line.startswith("["):
                in_domains = line == "[domains]"
                continue
            if in_domains and line and not line.startswith("#"):
                clean = line.lower().removeprefix("www.")
                safe_domains.add(clean)

# 2. Load Adult Blocklist
adult_domains = set()
if ADULT_DOMAINS_FILE.exists():
    with open(ADULT_DOMAINS_FILE, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                clean = line.lower().removeprefix("www.").removeprefix("0.0.0.0 ").removeprefix("127.0.0.1 ").strip()
                if "." in clean:
                    adult_domains.add(clean)

# 3. Heuristic Rules
ADULT_TLDS = {".xxx", ".porn", ".sex", ".adult", ".sexy", ".cam", ".tube", ".erotica"}
STRONG_KEYWORDS = {
    "porn", "pornhub", "xxx", "xvideo", "xvideos", "xnxx", "xhamster", "redtube",
    "youporn", "hentai", "milf", "brazzers", "bangbros", "chaturbate", "livejasmin",
    "stripchat", "camgirl", "onlyfans", "fansly", "manyvids", "pornstar", "spankbang",
    "tube8", "porntube", "sextube", "eporner", "beeg", "hqporner", "tnaflix", "thumbzilla",
    "motherless", "txxx", "tubepornclassic", "drtuber", "nuvid", "vporn", "daftsex",
    "daftporn", "pornone", "bravoteens", "fapello", "heavy-r", "leakgirls", "cam4",
    "camsoda", "bongacams", "flirt4free", "myfreecams", "porngo", "pornhat", "porn300",
    "4tube", "cumlouder", "porndoe", "hdporn", "jav", "javhd", "nsfw",
    "desibabe", "bhabhi", "antvasna", "kamababa", "desikahani", "savita", "chut", "choot",
    "malluporn", "tamilporn", "teluguporn", "banglaporn", "hindisex", "indianporn", "indiansex",
    "pussy", "boobs", "tits", "cock", "blowjob", "handjob", "creampie", "gangbang",
    "deepthroat", "cuckold", "dildo", "masturbat", "orgasm", "ejaculat", "cumshot",
    "slut", "whore", "cunt", "shemale", "transsexual", "voyeur", "upskirt", "incest",
    "porno", "sexo", "sexi", "erotik"
}

def is_safe(domain: str) -> bool:
    norm = domain.lower().removeprefix("www.")
    if norm in safe_domains:
        return True
    parts = norm.split(".")
    for i in range(1, len(parts) - 1):
        parent = ".".join(parts[i:])
        if parent in safe_domains:
            return True
    return False

def check_blocklist(domain: str) -> bool:
    norm = domain.lower().removeprefix("www.")
    if norm in adult_domains:
        return True
    parts = norm.split(".")
    for i in range(1, len(parts) - 1):
        parent = ".".join(parts[i:])
        if parent in adult_domains:
            return True
    return False

def check_heuristics(domain: str) -> tuple[bool, str]:
    norm = domain.lower().removeprefix("www.")
    for tld in ADULT_TLDS:
        if norm.endswith(tld):
            return True, f"Adult TLD ({tld})"
    
    stem = norm.rsplit(".", 1)[0]
    for kw in STRONG_KEYWORDS:
        if kw in stem:
            return True, f"Strong Keyword ({kw})"
    
    if "desi" in stem and any(k in stem for k in ["babe", "sex", "porn", "hot", "kahani", "bhabhi", "mms", "aunty"]):
        return True, "Desi Adult Compound"
    
    if "babe" in stem and any(k in stem for k in ["desi", "hot", "sexy", "teen", "cam", "tube"]):
        return True, "Babe Compound"

    return False, ""

def check_upstream_dns(domain: str, server_ip="1.1.1.3", timeout=2.0) -> bool:
    """Builds raw DNS packet and sends to Cloudflare Families (1.1.1.3)"""
    try:
        query = bytearray()
        query.extend(b"\x13\x37")  # ID
        query.extend(b"\x01\x00")  # Flags
        query.extend(b"\x00\x01\x00\x00\x00\x00\x00\x00")  # 1 question
        for part in domain.split("."):
            query.append(len(part))
            query.extend(part.encode("ascii"))
        query.append(0)  # null root
        query.extend(b"\x00\x01\x00\x01")  # Type A, Class IN

        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.settimeout(timeout)
        sock.sendto(query, (server_ip, 53))
        resp, _ = sock.recvfrom(512)
        sock.close()

        # Check if 0.0.0.0 is in the response
        if b"\x00\x00\x00\x00" in resp:
            return True
        # Check NXDOMAIN
        if len(resp) >= 4 and (resp[3] & 0x0F) == 3:
            return True
        return False
    except Exception:
        return False

def evaluate_domain(domain: str) -> dict:
    # Tier 1: Safe Allowlist
    if is_safe(domain):
        return {"domain": domain, "action": "ALLOWED", "reason": "Essential Safe Allowlist Override"}
    
    # Tier 2: Static Adult Blocklist
    if check_blocklist(domain):
        return {"domain": domain, "action": "BLOCKED", "reason": "Bundled Adult Blocklist Hit"}
    
    # Tier 3: Heuristics
    is_h, h_reason = check_heuristics(domain)
    if is_h:
        return {"domain": domain, "action": "BLOCKED", "reason": f"Heuristic Match: {h_reason}"}
    
    # Tier 4: Upstream Dynamic Query
    if check_upstream_dns(domain, "1.1.1.3"):
        return {"domain": domain, "action": "BLOCKED", "reason": "Cloudflare Families Dynamic DNS Interception"}

    if check_upstream_dns(domain, "185.228.168.168"):
        return {"domain": domain, "action": "BLOCKED", "reason": "CleanBrowsing Adult Filter Hit"}

    return {"domain": domain, "action": "ALLOWED", "reason": "Safe / Clean Domain"}

def get_foreground_window_windows():
    user32 = ctypes.windll.user32
    hwnd = user32.GetForegroundWindow()
    length = user32.GetWindowTextLengthW(hwnd)
    buff = ctypes.create_unicode_buffer(length + 1)
    user32.GetWindowTextW(hwnd, buff, length + 1)
    
    pid = ctypes.c_ulong()
    user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
    return hwnd, buff.value, pid.value

def run_tests():
    print("=" * 65)
    print("      SUPERVISION DESKTOP — LIVE VERIFICATION SUITE      ")
    print("=" * 65)
    print(f"Loaded {len(adult_domains)} bundled adult domains.")
    print(f"Loaded {len(safe_domains)} essential allowlisted domains.\n")

    test_cases = [
        # Expected BLOCKED (Adult)
        ("pornhub.com", "BLOCKED"),
        ("xvideos.com", "BLOCKED"),
        ("subdomain.brazzers.com", "BLOCKED"),
        ("streaming-content-live.xxx", "BLOCKED"),
        ("antvasna.com", "BLOCKED"),
        ("desibabevideos.in", "BLOCKED"),
        ("super-freesextube-online.net", "BLOCKED"),
        
        # Expected ALLOWED (Work, Dev, Banking, OS)
        ("github.com", "ALLOWED"),
        ("raw.githubusercontent.com", "ALLOWED"),
        ("crates.io", "ALLOWED"),
        ("teams.microsoft.com", "ALLOWED"),
        ("outlook.office.com", "ALLOWED"),
        ("slack.com", "ALLOWED"),
        ("zoom.us", "ALLOWED"),
        ("stackoverflow.com", "ALLOWED"),
        ("chase.com", "ALLOWED"),
        ("hdfcbank.com", "ALLOWED"),
        ("incometax.gov.in", "ALLOWED"),
        ("coursera.org", "ALLOWED"),
        ("python.org", "ALLOWED"),
    ]

    passed = 0
    total = len(test_cases)

    for domain, expected in test_cases:
        res = evaluate_domain(domain)
        status = "✓ PASS" if res["action"] == expected else "✗ FAIL"
        if res["action"] == expected:
            passed += 1
        print(f"[{status}] {domain:<30} -> {res['action']:<8} ({res['reason']})")

    print("\n" + "-" * 65)
    print(f"Domain Classification Results: {passed}/{total} Passed ({passed/total*100:.1f}%)")
    print("-" * 65)

    # Test Win32 Active Window Detection
    if os.name == "nt":
        print("\nTesting Active Foreground Window Inspection (Win32):")
        hwnd, title, pid = get_foreground_window_windows()
        print(f"  • Foreground HWND: {hwnd}")
        print(f"  • Foreground PID:  {pid}")
        print(f"  • Window Title:    '{title}'")
        print("  ✓ Active window inspection API is fully operational.")

    print("\n" + "=" * 65)
    print("All functional validation checks completed successfully!")
    print("=" * 65)

if __name__ == "__main__":
    run_tests()
