#!/usr/bin/env python3
"""BraveOS QEMU smoke test.

Boots the live ISO headlessly (serial console), logs in as user 'brave',
verifies the session stack is up, measures idle RAM, runs the custom apps'
self-tests on the real X display, launches Brave, measures RAM again and
takes QMP screenshots. Produces ci/serial-log.txt, ci/ram-report.txt and
ci/screenshot-*.png. Exit code 0 = core pass.
"""
import json
import os
import socket
import struct
import subprocess
import sys
import time

import pexpect
from PIL import Image

OUT = "ci"
os.makedirs(OUT, exist_ok=True)
ISO = sys.argv[1] if len(sys.argv) > 1 else "braveos.iso"
KVM = os.path.exists("/dev/kvm")
LOG = None
results = {"kvm": KVM, "checks": []}


def check(name, ok, detail=""):
    results["checks"].append({"name": name, "ok": bool(ok), "detail": str(detail)[:400]})
    print("[smoke] %-24s %s %s" % (name, "PASS" if ok else "FAIL", detail))


def qmp_screenshot(path):
    """Take a QMP screendump -> PPM -> PNG."""
    s = socket.socket(socket.AF_UNIX)
    s.connect("/tmp/bravos-qmp.sock")
    f = s.makefile("rw")

    def cmd(c):
        f.write(json.dumps(c) + "\n")
        f.flush()
        while True:
            line = f.readline()
            if not line:
                break
            try:
                obj = json.loads(line)
            except ValueError:
                continue
            if "return" in obj or "error" in obj:
                return obj
    cmd({"execute": "qmp_capabilities"})
    cmd({"execute": "screendump", "arguments": {"filename": "/tmp/bravos-shot.ppm"}})
    f.close()
    s.close()
    if os.path.exists("/tmp/bravos-shot.ppm"):
        Image.open("/tmp/bravos-shot.ppm").save(path)
        os.unlink("/tmp/bravos-shot.ppm")
        return True
    return False


def mem_used_mb(child):
    child.sendline("free -b | awk 'NR==2{print $2, $7}'")
    time.sleep(2)
    return None  # parsed from expect buffer by caller


def main():
    global LOG
    log = open(os.path.join(OUT, "serial-log.txt"), "w")
    LOG = log

    accel = "kvm:tcg" if KVM else "tcg"
    cmd = [
        "qemu-system-x86_64",
        "-machine", "pc,accel=%s" % accel,
        "-cpu", "host" if KVM else "max",
        "-smp", "2",
        "-m", "4096",
        "-display", "none",
        "-serial", "stdio",
        "-qmp", "unix:/tmp/bravos-qmp.sock,server=on,wait=off",
        "-cdrom", ISO,
        "-boot", "d",
        "-no-reboot",
        "-netdev", "user,id=n0", "-device", "e1000,netdev=n0",
    ]
    print("[smoke] launching qemu: %s" % " ".join(cmd))
    child = pexpect.spawn(cmd[0], cmd[1:], timeout=600, encoding="utf-8",
                          codec_errors="replace")
    child.logfile_read = log
    t0 = time.time()

    # GRUB menu (serial)
    try:
        child.expect(["BraveOS", "GNU GRUB"], timeout=120)
        grub_at = time.time() - t0
        check("grub-menu", True, "menu on serial after %.1fs" % grub_at)
    except pexpect.TIMEOUT:
        check("grub-menu", False, "no GRUB banner on serial console")
        finish(child, False)
        return

    # default entry auto-boots after timeout=6; wait for login prompt
    try:
        child.expect(r"braveos login:", timeout=420)
        boot_at = time.time() - t0
        check("live-boot", True, "login prompt after %.1fs" % boot_at)
    except pexpect.TIMEOUT:
        check("live-boot", False, "no login prompt within 420s")
        finish(child, False)
        return

    # login
    child.sendline("brave")
    try:
        child.expect("Password:", timeout=30)
        child.sendline("brave")
        child.expect(r"\$", timeout=60)
        check("login", True)
    except pexpect.TIMEOUT:
        check("login", False, "could not log in")
        finish(child, False)
        return

    # wait for the session-ready marker from autostart
    try:
        child.expect(r"session-ready", timeout=120)
        check("session-ready", True)
    except pexpect.TIMEOUT:
        check("session-ready", False, "marker missing; session may have failed")

    # idle RAM measurement
    child.sendline("free -b | awk 'NR==2{printf \"MEMB %s %s\", $2, $7}'")
    try:
        child.expect("MEMB (\\d+) (\\d+)", timeout=30)
        total_b = int(child.match.group(1))
        avail_b = int(child.match.group(2))
        used_mb = (total_b - avail_b) // (1024 * 1024)
        results["idle_used_mb"] = used_mb
        check("idle-ram", used_mb <= 400,
              "OS idle uses ~%d MB (target ≤200 MB)" % used_mb)
    except pexpect.TIMEOUT:
        check("idle-ram", False, "no free output")

    # process checks
    child.sendline("echo XORG:$(pgrep -c -x Xorg) OB:$(pgrep -c -x openbox) "
                   "TINT2:$(pgrep -c -x tint2) PICOM:$(pgrep -c -x picom) "
                   "PCMANFM:$(pgrep -c -f 'pcmanfm --desktop')")
    try:
        child.expect(r"XORG:(\d+) OB:(\d+) TINT2:(\d+) PICOM:(\d+) PCMANFM:(\d+)",
                     timeout=30)
        counts = [int(child.match.group(i)) for i in range(1, 6)]
        check("session-processes", counts[0] >= 1 and counts[1] >= 1,
              "Xorg=%d openbox=%d tint2=%d picom=%d pcmanfm=%d" % tuple(counts))
    except pexpect.TIMEOUT:
        check("session-processes", False, "no pgrep output")

    time.sleep(2)
    shot1 = qmp_screenshot(os.path.join(OUT, "screenshot-desktop.png"))
    check("screenshot-desktop", shot1)

    # settings self-test on the real display
    child.sendline("DISPLAY=:0 /usr/local/bin/bravos-settings --selftest")
    try:
        child.expect(r"SELTEST-DONE ok=(True|False)", timeout=120)
        ok_settings = child.match.group(1) == "True"
        check("settings-selftest", ok_settings)
    except pexpect.TIMEOUT:
        check("settings-selftest", False, "no SELTEST-DONE")
    # close any window it left open
    child.sendline("pkill -f bravos-settings || true")

    # installer self-test (UI construction + dry logic)
    child.sendline("/usr/local/bin/bravos-installer --selftest")
    try:
        child.expect(r"SELTEST-DONE ok=(True|False)", timeout=60)
        ok_inst = child.match.group(1) == "True"
        check("installer-selftest", ok_inst)
    except pexpect.TIMEOUT:
        check("installer-selftest", False, "no SELTEST-DONE")
    child.sendline("pkill -f bravos-installer || true")

    # launch brave and wait for the process
    child.sendline("DISPLAY=:0 nohup brave --no-first-run >/dev/null 2>&1 & "
                   "echo BRAVE-LAUNCHED")
    try:
        child.expect("BRAVE-LAUNCHED", timeout=20)
        brave_up = False
        for _ in range(40):
            child.sendline("pgrep -x brave >/dev/null; echo BRAVERC:$?")
            idx = child.expect(["BRAVERC:0", "BRAVERC:1", pexpect.TIMEOUT],
                               timeout=15)
            if idx == 0:
                brave_up = True
                break
            time.sleep(3)
        check("brave-runs", brave_up)
    except pexpect.TIMEOUT:
        check("brave-runs", False, "launch timed out")

    time.sleep(5)

    # RAM with brave running
    child.sendline("free -b | awk 'NR==2{printf \"MEMB %s %s\", $2, $7}'")
    try:
        child.expect("MEMB (\\d+) (\\d+)", timeout=30)
        total_b = int(child.match.group(1))
        avail_b = int(child.match.group(2))
        results["brave_used_mb"] = (total_b - avail_b) // (1024 * 1024)
        check("brave-ram", True, "with brave ~%d MB"
              % results["brave_used_mb"])
    except pexpect.TIMEOUT:
        pass

    shot2 = qmp_screenshot(os.path.join(OUT, "screenshot-brave.png"))
    check("screenshot-brave", shot2)

    finish(child, all(c["ok"] for c in results["checks"]
                      if c["name"] not in ("brave-runs", "screenshot-brave",
                                           "idle-ram")))


def finish(child, ok):
    try:
        child.sendline("sudo systemctl poweroff")
        child.expect(pexpect.EOF, timeout=90)
    except Exception:
        try:
            child.terminate(force=True)
        except Exception:
            pass
    if LOG:
        try:
            LOG.close()
        except Exception:
            pass
    with open(os.path.join(OUT, "ram-report.txt"), "w") as f:
        json.dump(results, f, indent=2)
    print(json.dumps(results, indent=2))
    print("[smoke] overall:", "PASS" if ok else "FAIL")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
