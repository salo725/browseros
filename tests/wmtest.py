#!/usr/bin/env python3
"""wmtest.py - automated button-level test driver for BraveWM + the BraveOS bar.

Runs INSIDE one sandbox call (the sandbox reaps background processes between
calls): start Xvfb + bravos-wm, then run this in the same invocation.

Synthetic mouse input uses XSendEvent targeted at the bar window (the WM does
not filter sent events). State is asserted through root-window properties:
  _BRAVEOS_BAR_ZONES                bar hit zones, "id:x,y,w,h;..."
  _NET_CURRENT_DESKTOP / _NET_NUMBER_OF_DESKTOPS
Launches are verified through PATH-shim fake apps appending to a log file.

usage: wmtest.py <display> <shotdir> <spawnlog>
"""
import ctypes
import os
import sys
import time

X11 = ctypes.CDLL("libX11.so.6")
X11.XOpenDisplay.restype = ctypes.c_void_p
X11.XOpenDisplay.argtypes = [ctypes.c_char_p]
X11.XDefaultRootWindow.restype = ctypes.c_ulong
X11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
X11.XCloseDisplay.argtypes = [ctypes.c_void_p]
X11.XFlush.argtypes = [ctypes.c_void_p]
X11.XSync.argtypes = [ctypes.c_void_p, ctypes.c_int]
X11.XInternAtom.restype = ctypes.c_ulong
X11.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
X11.XQueryTree.restype = ctypes.c_int
X11.XQueryTree.argtypes = [ctypes.c_void_p, ctypes.c_ulong,
                           ctypes.POINTER(ctypes.c_ulong),
                           ctypes.POINTER(ctypes.c_ulong),
                           ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)),
                           ctypes.POINTER(ctypes.c_uint)]


class XButtonEvent(ctypes.Structure):
    _fields_ = [("type", ctypes.c_int), ("serial", ctypes.c_ulong),
                ("send_event", ctypes.c_int), ("display", ctypes.c_void_p),
                ("window", ctypes.c_ulong), ("root", ctypes.c_ulong),
                ("subwindow", ctypes.c_ulong), ("time", ctypes.c_ulong),
                ("x", ctypes.c_int), ("y", ctypes.c_int),
                ("x_root", ctypes.c_int), ("y_root", ctypes.c_int),
                ("state", ctypes.c_uint), ("button", ctypes.c_uint),
                ("same_screen", ctypes.c_int)]


BUTTON_PRESS = 4
BUTTON_RELEASE = 5
MOTION_NOTIFY = 6

PASS = 0
FAIL = 0


def check(name, ok, detail=""):
    global PASS, FAIL
    print("[wmtest] %-52s %s %s" % (name, "PASS" if ok else "FAIL", detail))
    if ok:
        PASS += 1
    else:
        FAIL += 1


class XWindowAttributes(ctypes.Structure):
    _fields_ = [("x", ctypes.c_int), ("y", ctypes.c_int),
                ("width", ctypes.c_int), ("height", ctypes.c_int),
                ("border_width", ctypes.c_int), ("depth", ctypes.c_int),
                ("visual", ctypes.c_void_p), ("root", ctypes.c_ulong),
                ("c_class", ctypes.c_int), ("bit_gravity", ctypes.c_int),
                ("win_gravity", ctypes.c_int), ("backing_store", ctypes.c_int),
                ("backing_planes", ctypes.c_ulong),
                ("backing_pixel", ctypes.c_ulong), ("save_under", ctypes.c_int),
                ("colormap", ctypes.c_ulong), ("map_installed", ctypes.c_int),
                ("map_state", ctypes.c_int),
                ("all_event_masks", ctypes.c_long),
                ("your_event_mask", ctypes.c_long),
                ("do_not_propagate", ctypes.c_long),
                ("override_redirect", ctypes.c_int), ("screen", ctypes.c_void_p)]


X11.XGetWindowAttributes.restype = ctypes.c_int
X11.XGetWindowAttributes.argtypes = [ctypes.c_void_p, ctypes.c_ulong,
                                      ctypes.POINTER(XWindowAttributes)]


class Tester:
    def __init__(self, display):
        self.dpy = X11.XOpenDisplay(display.encode())
        assert self.dpy, "cannot open display " + display
        self.root = X11.XDefaultRootWindow(self.dpy)

    def atom(self, name):
        return X11.XInternAtom(self.dpy, name.encode(), False)

    def prop_text(self, win, name):
        """Read an 8-bit text property (XA_STRING)."""
        atom = self.atom(name)
        t = ctypes.c_ulong()
        f = ctypes.c_int()
        n = ctypes.c_ulong()
        a = ctypes.c_ulong()
        data = ctypes.POINTER(ctypes.c_ubyte)()
        rc = X11.XGetWindowProperty(
            self.dpy, win, atom, 0, 8192, False, 31,  # XA_STRING
            ctypes.byref(t), ctypes.byref(f), ctypes.byref(n), ctypes.byref(a),
            ctypes.byref(data))
        if rc != 0 or not data or n.value == 0:
            return None
        raw = (ctypes.c_ubyte * min(n.value, 8192)).from_address(
            ctypes.addressof(data.contents))
        return bytes(raw).decode("utf-8", "replace").rstrip("\x00")

    def prop_long(self, win, name):
        """Read a CARDINAL (32-bit) property as int."""
        atom = self.atom(name)
        t = ctypes.c_ulong()
        f = ctypes.c_int()
        n = ctypes.c_ulong()
        a = ctypes.c_ulong()
        data = ctypes.POINTER(ctypes.c_ubyte)()
        rc = X11.XGetWindowProperty(
            self.dpy, win, atom, 0, 4, False, 6,  # XA_CARDINAL
            ctypes.byref(t), ctypes.byref(f), ctypes.byref(n), ctypes.byref(a),
            ctypes.byref(data))
        if rc != 0 or not data or n.value == 0:
            return None
        val = ctypes.cast(data, ctypes.POINTER(ctypes.c_long)).contents.value
        return val

    def send_button(self, win, x, y, button=1, release=False):
        ev = XButtonEvent()
        ctypes.memset(ctypes.byref(ev), 0, ctypes.sizeof(ev))
        ev.type = BUTTON_RELEASE if release else BUTTON_PRESS
        ev.window = win
        ev.root = self.root
        ev.time = int(time.time() * 1000) & 0xFFFFFFFF
        ev.x, ev.y = x, y
        ev.x_root, ev.y_root = x, y
        ev.button = button
        ev.same_screen = 1
        X11.XSendEvent(self.dpy, win, False, 0, ctypes.byref(ev))
        X11.XFlush(self.dpy)

    def bar_zones(self):
        s = self.prop_text(self.root, "_BRAVEOS_BAR_ZONES")
        zones = {}
        if not s:
            return zones
        for part in s.split(";"):
            if not part or ":" not in part:
                continue
            zid, geom = part.split(":", 1)
            try:
                x, y, w, h = geom.split(",")
                zones[int(zid)] = tuple(int(v) for v in (x, y, w, h))
            except ValueError:
                continue
        return zones

    def find_bar_window(self):
        rroot = ctypes.c_ulong()
        parent = ctypes.c_ulong()
        children = ctypes.POINTER(ctypes.c_ulong)()
        n = ctypes.c_uint()
        if not X11.XQueryTree(self.dpy, self.root, ctypes.byref(rroot),
                               ctypes.byref(parent), ctypes.byref(children),
                               ctypes.byref(n)):
            return None, []
        wins = []
        bar = None
        if children and n.value:
            for i in range(n.value):
                w = children[i]
                wa = XWindowAttributes()
                if X11.XGetWindowAttributes(self.dpy, w, ctypes.byref(wa)):
                    wins.append((w, wa))
                    if (wa.override_redirect and wa.map_state != 0 and
                            wa.width > 400 and 20 <= wa.height <= 64 and wa.y <= 4):
                        bar = w
        return bar, wins


def read_spawn_log(path):
    try:
        with open(path) as f:
            return f.read()
    except OSError:
        return ""


def main():
    display = sys.argv[1] if len(sys.argv) > 1 else ":99"
    spawnlog = sys.argv[2] if len(sys.argv) > 2 else "/tmp/spawn.log"
    t = Tester(display)

    bar_win, wins = t.find_bar_window()
    check("bar window found", bar_win is not None,
          "win=0x%x" % bar_win if bar_win else "")
    zones = t.bar_zones()
    check("bar zones exported", len(zones) >= 5, "%d zones" % len(zones))
    if not bar_win or not zones:
        print("[wmtest] PASS=%d FAIL=%d" % (PASS, FAIL))
        return 1

    bar_y = zones[10][1] if 10 in zones else 0

    def click_zone(zid, button=1):
        z = zones.get(zid)
        if not z:
            check("zone %d exists" % zid, False)
            return
        x, y, w, h = z
        cx, cy = x + w // 2, y + h // 2
        t.send_button(bar_win, cx, cy - bar_y, button)
        time.sleep(0.2)
        t.send_button(bar_win, cx, cy - bar_y, button, release=True)
        time.sleep(0.3)

    # ---- initial state ----
    check("initial desktops = 2",
          t.prop_long(t.root, "_NET_NUMBER_OF_DESKTOPS") == 2,
          str(t.prop_long(t.root, "_NET_NUMBER_OF_DESKTOPS")))
    check("initial current = 0",
          t.prop_long(t.root, "_NET_CURRENT_DESKTOP") == 0)

    # ---- pill 2 -> desktop 1 ----
    click_zone(11)
    check("pill '2' click -> current desktop 1",
          t.prop_long(t.root, "_NET_CURRENT_DESKTOP") == 1,
          str(t.prop_long(t.root, "_NET_CURRENT_DESKTOP")))

    # ---- '+' -> 3 desktops ----
    click_zone(200)
    check("'+' click -> 3 desktops",
          t.prop_long(t.root, "_NET_NUMBER_OF_DESKTOPS") == 3,
          str(t.prop_long(t.root, "_NET_NUMBER_OF_DESKTOPS")))

    # ---- pill 1 -> back to desktop 0 ----
    click_zone(10)
    check("pill '1' click -> back to desktop 0",
          t.prop_long(t.root, "_NET_CURRENT_DESKTOP") == 0,
          str(t.prop_long(t.root, "_NET_CURRENT_DESKTOP")))

    # ---- status zones: launch apps via PATH shims; zones adapt to hardware
    log = read_spawn_log(spawnlog)
    for zid, want in ((300, "settings --page network"),
                      (302, "settings --page power")):
        if zid in zones:
            click_zone(zid)
            time.sleep(0.3)
        else:
            print("[wmtest] %-52s SKIP (hardware absent, icon hidden)" %
                  ("zone %d (%s)" % (zid, want.split()[-1])))
    log = read_spawn_log(spawnlog)
    if 300 in zones:
        check("Wi-Fi icon -> opens Network settings",
              "settings --page network" in log)
    if 302 in zones:
        check("Battery icon -> opens Power settings",
              "settings --page power" in log)

    # ---- power menu opens ----
    click_zone(304)
    time.sleep(0.5)
    _, wins2 = t.find_bar_window()
    menu_found = any(wa.override_redirect and wa.map_state != 0 and
                     60 < wa.height < 420 and wa.width > 80
                     for _, wa in wins2)
    check("power button -> power menu window appears", menu_found)

    # ---- power menu item 5 (shut down) via click inside menu window ----
    if menu_found:
        menu_win = [w for w, wa in wins2 if wa.override_redirect and
                    wa.map_state != 0 and 60 < wa.height < 420 and
                    wa.width > 80][0]
        mwa = [wa for w, wa in wins2 if w == menu_win][0]
        my = mwa.y + mwa.height - 25  # last item = Shut Down
        t.send_button(menu_win, mwa.x + mwa.width // 2 - mwa.x, my, 1)
        time.sleep(0.4)
        t.send_button(menu_win, mwa.x + mwa.width // 2 - mwa.x, my, 1, True)
        time.sleep(0.4)
        log = read_spawn_log(spawnlog)
        check("power menu 'Shut Down' -> bravos-power poweroff",
              "bravos-power poweroff" in log, log.strip().replace("\n", " | "))

    print("[wmtest] PASS=%d FAIL=%d" % (PASS, FAIL))
    return 1 if FAIL else 0


if __name__ == "__main__":
    sys.exit(main())
