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
import re
import shutil
import socket
import struct
import subprocess
import sys
import time

import pexpect
from PIL import Image

ISO = sys.argv[1] if len(sys.argv) > 1 else "braveos.iso"
MODE = sys.argv[2] if len(sys.argv) > 2 else "cd"
OUT = {"cd": "ci", "disk": "ci-disk", "uefi": "ci-uefi",
       "fatbios": "ci-fatbios", "fatuefi": "ci-fatuefi"}.get(MODE, "ci")
os.makedirs(OUT, exist_ok=True)


def kvm_usable():
    """/dev/kvm must exist AND be writable by this user."""
    if not os.path.exists("/dev/kvm"):
        return False
    try:
        fd = os.open("/dev/kvm", os.O_RDWR)
        os.close(fd)
        return True
    except OSError:
        return False


KVM = kvm_usable()
LOG = None
results = {"kvm": KVM, "checks": []}


def mem_from_log():
    """Parse the last MEMB total/avail pair from the captured serial log.

    Parsing the log file directly (plain re) is immune to the pexpect
    buffer-position quirks seen on timed-out expects.
    """
    try:
        pairs = re.findall(r"MEMB (\d+) (\d+)",
                           open(os.path.join(OUT, "serial-log.txt"),
                                errors="replace").read())
    except OSError:
        return None
    return pairs[-1] if pairs else None


def wait_for_session_ready(timeout_s):
    """The session writes 'session-ready' to kmsg during boot (often BEFORE
    the login prompt), so match it against the whole captured serial log."""
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            if "session-ready" in open(os.path.join(OUT, "serial-log.txt"),
                                       errors="replace").read():
                return True
        except OSError:
            pass
        time.sleep(2)
    return False


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
    """Idle RAM from the IDLERAM marker the guest ui-test prints."""
    try:
        log = open(os.path.join(OUT, "serial-log.txt"), errors="replace").read()
        pairs = re.findall(r"IDLERAM (\d+) (\d+)", log)
        if pairs:
            total, avail = int(pairs[0][0]), int(pairs[0][1])
            return (total - avail) // (1024 * 1024)
    except OSError:
        pass
    return None


def main():
    global LOG
    os.makedirs(OUT, exist_ok=True)
    log = open(os.path.join(OUT, "serial-log.txt"), "w", buffering=1)
    LOG = log

    accel = "kvm" if KVM else "tcg"
    cmd = [
        "qemu-system-x86_64",
        "-machine", "pc,accel=%s" % accel,
        "-cpu", "host" if KVM else "max",
        "-smp", "2",
        "-m", "4096",
        "-display", "none",
        "-serial", "stdio",
        "-qmp", "unix:/tmp/bravos-qmp.sock,server=on,wait=off",
        "-no-reboot",
        "-netdev", "user,id=n0", "-device", "e1000,netdev=n0",
    ]
    if MODE == "cd":
        cmd += ["-cdrom", ISO, "-boot", "d"]
    elif MODE in ("fatbios", "fatuefi"):
        # Rufus "ISO Image" mode emulation: extract the ISO onto a FAT32
        # volume; BIOS boots it via the isolinux boot sector (installed with
        # syslinux, exactly what Rufus does), UEFI boots /EFI/boot/bootx64.efi.
        import tempfile
        fat = os.path.join(OUT, "fat.img")
        subprocess.run(["dd", "if=/dev/zero", "of=%s" % fat, "bs=1M",
                        "seek=2600", "count=1"], check=True)
        subprocess.run(["mkfs.vfat", "-F", "32", fat], check=True)
        m = tempfile.mkdtemp(prefix="bravosfat")
        subprocess.run(["mount", "-o", "loop,uid=$(id -u)", fat, m], check=True)
        subprocess.run(["7z", "x", "-y", "-o%s" % m, ISO],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if MODE == "fatbios":
            dev = os.popen("losetup --list --output NAME "
                           "--associate %s | tail -1" % fat).read().strip()
            part = dev + "p1" if dev and os.path.exists(dev + "p1") else dev
            subprocess.run(["syslinux", "--install", part or dev], check=True)
        subprocess.run(["umount", m], check=True)
        cmd += ["-drive", "file=%s,format=raw,media=disk" % fat, "-boot", "c"]
        if MODE == "fatuefi":
            ovmf = None
            for p in ("/usr/share/OVMF/OVMF.fd",
                      "/usr/share/OVMF/OVMF_CODE.fd",
                      "/usr/share/qemu/OVMF.fd"):
                if os.path.exists(p):
                    ovmf = p
                    break
            if not ovmf:
                check("ovmf-present", False, "no OVMF firmware found")
                finish(None, False)
                return
            cmd = ["qemu-system-x86_64", "-bios", ovmf] + cmd[1:]
    else:
        # USB scenarios: boot the ISO as a raw disk the way a flashed stick
        # presents itself (BIOS via MBR hybrid / UEFI via the ESP).
        img = os.path.join(OUT, "usb.img")
        print("[smoke] copying ISO to raw disk image %s" % img)
        shutil.copyfile(ISO, img)
        cmd += ["-drive", "file=%s,format=raw,media=disk" % img,
                "-boot", "c"]
        if MODE == "uefi":
            ovmf = None
            for p in ("/usr/share/OVMF/OVMF.fd",
                      "/usr/share/OVMF/OVMF_CODE.fd",
                      "/usr/share/qemu/OVMF.fd"):
                if os.path.exists(p):
                    ovmf = p
                    break
            if not ovmf:
                check("ovmf-present", False, "no OVMF firmware found")
                finish(None, False)
                return
            check("ovmf-present", True, ovmf)
            cmd = ["qemu-system-x86_64", "-bios", ovmf] + cmd[1:]
    print("[smoke] launching qemu: %s" % " ".join(cmd))
    child = pexpect.spawn(cmd[0], cmd[1:], timeout=600, encoding="utf-8",
                          codec_errors="replace")
    child.logfile_read = log
    t0 = time.time()

    # GRUB menu (serial). TCG (no KVM) emulation is slow: be generous.
    try:
        child.expect(["BraveOS", "GNU GRUB"], timeout=300)
        grub_at = time.time() - t0
        check("grub-menu", True, "menu on serial after %.1fs" % grub_at)
    except pexpect.TIMEOUT:
        check("grub-menu", False, "no GRUB banner on serial console")
        finish(child, False)
        return

    # default entry auto-boots after timeout=6; wait for login prompt
    try:
        child.expect(r"braveos login:", timeout=900)
        boot_at = time.time() - t0
        check("live-boot", True, "login prompt after %.1fs" % boot_at)
    except pexpect.TIMEOUT:
        check("live-boot", False, "no login prompt within 900s")
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
    # session-ready: written to kmsg during boot; poll the captured log
    session_ok = wait_for_session_ready(150)
    check("session-ready", session_ok,
          "" if session_ok else "marker missing; session may have failed")

    # idle RAM is measured inside ui-test (IDLERAM marker, printed before any
    # app launches) — checked after ui-functional below.

    # process checks (retry: bwm/bshell start right after the marker, so the
    # first pgrep may race ahead of them - up to ~25s)
    counts = None
    for attempt in range(10):
        child.sendline("echo XORG:$(pgrep -c -x Xorg) BWM:$(pgrep -c -x bwm) "
                       "BSHELL:$(pgrep -c -x bshell) BWALL:$(pgrep -c -x bwall) "
                       "PICOM:$(pgrep -c -x picom)")
        try:
            child.expect(r"XORG:(\d+) BWM:(\d+) BSHELL:(\d+) BWALL:(\d+) PICOM:(\d+)",
                         timeout=20)
            counts = [int(child.match.group(i)) for i in range(1, 6)]
            if counts[0] >= 1 and counts[1] >= 1 and counts[2] >= 1 and counts[3] >= 1:
                break
        except pexpect.TIMEOUT:
            counts = None
        time.sleep(2.5)
    if counts:
        check("session-processes",
              counts[0] >= 1 and counts[1] >= 1 and counts[2] >= 1 and counts[3] >= 1,
              "Xorg=%d bwm=%d bshell=%d bwall=%d picom=%d" % tuple(counts))
    else:
        check("session-processes", False, "no pgrep output")

    time.sleep(2)
    shot1 = qmp_screenshot(os.path.join(OUT, "screenshot-desktop.png"))
    check("screenshot-desktop", shot1)

    # UI functional test: real interactions on the live desktop
    child.sendline("bash /usr/local/bin/ui-test.sh; echo UITEST-RC:$?")
    try:
        child.expect(r"UITEST-DONE pass=(\d+) fail=(\d+)", timeout=420)
        ui_pass, ui_fail = int(child.match.group(1)), int(child.match.group(2))
        check("ui-functional", ui_fail == 0,
              "pass=%d fail=%d" % (ui_pass, ui_fail))
    except pexpect.TIMEOUT:
        check("ui-functional", False, "no UITEST-DONE (crash or hang)")
    child.sendline("pkill -x bsettings 2>/dev/null; pkill -x xterm 2>/dev/null; true")

    # pull the guest's diagnostic screenshots (UITEST-SHOT base64 blocks)
    import base64 as _b64
    try:
        log_data = open(os.path.join(OUT, "serial-log.txt"), errors="replace").read()
        for m in re.finditer(r"UITEST-SHOT-BEGIN (\S+)\n([A-Za-z0-9+/=\s]+?)\nUITEST-SHOT-END \1",
                             log_data):
            name, data = m.group(1), m.group(2).replace("\n", "").replace(" ", "")
            try:
                png = _b64.b64decode(data)
                path = os.path.join(OUT, "uitest-%s.png" % name)
                open(path, "wb").write(png)
                print("[smoke] pulled guest screenshot %s (%d KB)" % (name, len(png) // 1024))
            except Exception as e:
                print("[smoke] screenshot %s decode failed: %s" % (name, e))
    except OSError:
        pass

    # idle RAM from the guest ui-test's IDLERAM marker (pre-launch state)
    pair = None
    deadline = time.time() + 30
    while time.time() < deadline and pair is None:
        pair = mem_used_mb(child)
        if pair is None:
            time.sleep(2)
    if pair:
        results["idle_used_mb"] = pair
        check("idle-ram", pair <= 600,
              "idle used=%dMB (live media incl. caches; target ≤600)" % pair)
    else:
        check("idle-ram", False, "no IDLERAM marker")

    # installer self-test (UI construction + dry logic, python/GTK app)
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

    # RAM with brave running (same log-file parsing as idle)
    child.sendline("free -b | awk 'NR==2{printf \"MEMB %s %s\", $2, $7}'")
    pair2 = None
    deadline = time.time() + 60
    while time.time() < deadline and pair2 is None:
        time.sleep(2)
        pair2 = mem_from_log()
    if pair2:
        total_b, avail_b = int(pair2[0]), int(pair2[1])
        results["brave_used_mb"] = (total_b - avail_b) // (1024 * 1024)
        check("brave-ram", True, "with brave ~%d MB"
              % results["brave_used_mb"])
    else:
        check("brave-ram", False, "no free output")

    shot2 = qmp_screenshot(os.path.join(OUT, "screenshot-brave.png"))
    check("screenshot-brave", shot2)

    finish(child, all(c["ok"] for c in results["checks"]
                      if c["name"] not in ("brave-runs", "screenshot-brave",
                                           "idle-ram", "installer-selftest")))


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
    # On failure, dump the serial tail into the job log so the failure
    # point is visible without downloading artifacts.
    if not ok:
        try:
            tail = open(os.path.join(OUT, "serial-log.txt"),
                        errors="replace").read().splitlines()[-160:]
            print("===== SERIAL TAIL (last %d lines) =====" % len(tail))
            print("\n".join(tail))
            print("===== END SERIAL TAIL =====")
        except OSError:
            print("(serial log unavailable)")
    # capture the guest's UITEST-MEM breakdown lines into the ram report
    mem_lines = []
    try:
        with open(os.path.join(OUT, "serial-log.txt"), errors="replace") as f:
            for ln in f:
                if "UITEST-MEM " in ln:
                    mem_lines.append(ln.split("UITEST-MEM ", 1)[1].rstrip())
    except OSError:
        pass

    with open(os.path.join(OUT, "ram-report.txt"), "w") as f:
        for ml in mem_lines:
            f.write(ml + "\n")
        f.write("\n")
        json.dump(results, f, indent=2)
    print(json.dumps(results, indent=2))
    print("[smoke] overall:", "PASS" if ok else "FAIL")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
