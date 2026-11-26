#!/usr/bin/env python3
"""docktest.py - automated button-level tests for bravos-dock WITHOUT a display.

Strategy: bravos-dock is a GTK3+cairo app, but its core logic (responsive
layout math, input-shape region building, hit testing, hover/press state,
button activation) is pure Python operating on plain numbers, and on_draw()
is pure cairo given a context. So we install a minimal mock of the `gi`
package (no-op widgets) before importing the dock, then:

  1. instantiate Dock at several screen sizes -> responsive layout assertions
  2. assert every button lies inside the window and the dome
  3. assert the X input shape region covers buttons but not outside areas
  4. drive motion/press/release -> hover states + PATH-shim launch commands
  5. render on_draw() into a real cairo surface -> PNG (for VLM review)

usage: docktest.py <out_png_dir> <spawnlog>
"""
import importlib.util
import math
import os
import subprocess
import sys
import time
import types

import cairo

PASS = 0
FAIL = 0


def check(name, ok, detail=""):
    global PASS, FAIL
    print("[docktest] %-52s %s %s" % (name, "PASS" if ok else "FAIL", detail))
    if ok:
        PASS += 1
    else:
        FAIL += 1


# ---------------------------------------------------------------- mock gi ----


class MockEvent:
    def __init__(self, x, y, direction=None, keyval=None):
        self.x = x
        self.y = y
        self.direction = direction
        self.keyval = keyval


class MockRequisition:
    def __init__(self, width=180, height=200):
        self.width = width
        self.height = height


class MockMonitorGeometry:
    def __init__(self, w, h):
        self.x = 0
        self.y = 0
        self.width = w
        self.height = h


class MockGdkWindow:
    def __init__(self):
        self.regions = []

    def get_pointer(self):
        return (60, 40, 0)

    def shape_combine_region(self, region, a, b):
        self.regions.append(("shape", region))

    def input_shape_combine_region(self, region, a, b):
        self.regions.append(("input", region))


class MockVisual:
    pass


MOCK_W = 1920
MOCK_H = 1080


class MockScreen:
    _default = None

    def __init__(self, w=MOCK_W, h=MOCK_H):
        self.w = w
        self.h = h

    def width(self):
        return self.w

    def height(self):
        return self.h

    def get_rgba_visual(self):
        return MockVisual()

    def get_primary_monitor(self):
        return 0

    def get_monitor_geometry(self, i):
        return MockMonitorGeometry(self.w, self.h)

    @staticmethod
    def get_default():
        if MockScreen._default is None:
            MockScreen._default = MockScreen()
        return MockScreen._default

    @staticmethod
    def set_size(w, h):
        MockScreen._default = MockScreen(w, h)


class MockWidget:
    def __init__(self, *a, **kw):
        self.handlers = {}
        self.alloc = (0, 0, 100, 40)
        self.style_classes = []
        self.visible = False

    # widget API
    def set_decorated(self, v): pass

    def set_app_paintable(self, v): pass

    def set_skip_taskbar_hint(self, v): pass

    def set_skip_pager_hint(self, v): pass

    def set_type_hint(self, v): pass

    def set_keep_above(self, v): pass

    def set_title(self, v): pass

    def set_name(self, v): self._name = v

    def set_events(self, v): pass

    def get_events(self):
        return 0

    def set_resizable(self, v): pass

    def set_visual(self, v): pass

    def set_border_width(self, v): pass

    def connect(self, signal, cb, *extra):
        self.handlers[signal] = (cb, extra)

    def get_screen(self):
        return MockScreen.get_default()

    def get_window(self):
        return getattr(self, "_gdkwin", None)

    def set_allocated_window(self, gdkwin):
        self._gdkwin = gdkwin

    def get_allocation(self):
        class A:
            width = self.alloc[2]
            height = self.alloc[3]
        return A()

    def get_allocated_width(self):
        return self.alloc[2]

    def get_allocated_height(self):
        return self.alloc[3]

    def get_preferred_size(self):
        return (MockRequisition(0, 0), MockRequisition(240, 260))

    def move(self, x, y):
        self._pos = (x, y)

    def resize(self, w, h):
        self._size = (w, h)

    def show_all(self):
        self.visible = True

    def destroy(self):
        pass

    def grab_add(self):
        pass

    def queue_draw(self):
        pass

    def set_size_request(self, w, h):
        self.alloc = (0, 0, w, h)

    def pack_start(self, w, expand, fill, pad):
        pass

    def add(self, w):
        pass

    # style
    def get_style_context(self):
        st = MockWidget()
        st.add_class = lambda c: None
        return st


class MockBox(MockWidget):
    def __init__(self, orientation=None, spacing=0, **kw):
        super().__init__(**kw)


class MockButton(MockWidget):
    def __init__(self, label=None, **kw):
        super().__init__(**kw)
        self.label = label


class MockSeparator(MockWidget):
    pass


class MockDrawingArea(MockWidget):
    pass


class MockLabel(MockWidget):
    def __init__(self, label=None, **kw):
        super().__init__(**kw)
        self.label = label

    def set_markup(self, m): pass

    def set_ellipsize(self, v): pass

    def set_max_width_chars(self, n): pass


class MockWindow(MockWidget):
    def __init__(self, type=None):
        super().__init__()
        self.type = type


class MockGtkModule(types.ModuleType):
    class WindowType:
        TOPLEVEL = "toplevel"
        POPUP = "popup"

    class Orientation:
        VERTICAL = "v"
        HORIZONTAL = "h"

    def __init__(self):
        super().__init__("gtkmock")
        self.Window = MockWindow
        self.Box = MockBox
        self.Button = MockButton
        self.Label = MockLabel
        self.Separator = MockSeparator
        self.DrawingArea = MockDrawingArea
        self.WindowType = self.WindowType
        self.Orientation = self.Orientation
        self.main = lambda: None
        self.main_quit = lambda: None

        class StyleContext:
            PRIORITY_APPLICATION = 600

            @staticmethod
            def add_provider_for_screen(screen, provider, prio):
                pass

        self.StyleContext = StyleContext

        class CssProvider:
            def load_from_data(self, data):
                pass

        self.CssProvider = CssProvider
        self.STYLE_PROVIDER_PRIORITY_APPLICATION = 600


class ScreenFacade:
    """Gdk.Screen-like class-level facade (Gdk.Screen.width() is a class
    method in PyGObject)."""

    @staticmethod
    def width():
        return MockScreen.get_default().width()

    @staticmethod
    def height():
        return MockScreen.get_default().height()

    @staticmethod
    def get_default():
        return MockScreen.get_default()


class MockGdkModule(types.ModuleType):
    class WindowTypeHint:
        DOCK = "dock"
        NORMAL = "normal"

    class EventMask:
        BUTTON_PRESS_MASK = 1
        BUTTON_RELEASE_MASK = 2
        POINTER_MOTION_MASK = 4
        LEAVE_NOTIFY_MASK = 8
        SCROLL_MASK = 16
        EXPOSURE_MASK = 32
        KEY_PRESS_MASK = 64

    class ScrollDirection:
        UP = "up"
        DOWN = "down"

    KEY_Escape = 65307

    def __init__(self):
        super().__init__("gdkmock")
        self.WindowTypeHint = self.WindowTypeHint
        self.EventMask = self.EventMask
        self.ScrollDirection = self.ScrollDirection
        self.Screen = ScreenFacade


class MockGlibModule(types.ModuleType):
    timeouts = []

    def __init__(self):
        super().__init__("glibmock")

    def timeout_add(self, ms, cb):
        MockGlibModule.timeouts.append((ms, cb))
        return len(MockGlibModule.timeouts)


class MockPangoModule(types.ModuleType):
    class EllipsizeMode:
        END = "end"

    def __init__(self):
        super().__init__("pangomock")
        self.EllipsizeMode = self.EllipsizeMode


def install_mock_gi():
    gi = types.ModuleType("gi")

    def require_version(*a, **kw):
        pass

    gi.require_version = require_version

    gtk_mod = MockGtkModule()
    gdk_mod = MockGdkModule()
    glib_mod = MockGlibModule()
    pango_mod = MockPangoModule()

    gi.repository = types.SimpleNamespace(
        Gtk=gtk_mod, Gdk=gdk_mod, GLib=glib_mod, Pango=pango_mod)

    sys.modules["gi"] = gi
    sys.modules["gi.repository"] = gi.repository
    return gi


def load_dock(path):
    from importlib.machinery import SourceFileLoader
    loader = SourceFileLoader("bravos_dock_mod", path)
    spec = importlib.util.spec_from_loader("bravos_dock_mod", loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


# ------------------------------------------------------------------ tests ----


def read_spawn_log(path):
    try:
        with open(path) as f:
            return f.read()
    except OSError:
        return ""


def test_at_size(mod, w, h, spawnlog, outpng):
    MockScreen.set_size(w, h)
    dock = mod.Dock()
    dock.show_all()
    gdkwin = MockGdkWindow()
    dock.set_allocated_window(gdkwin)
    dock.apply_input_shape()

    tag = "%dx%d" % (w, h)
    W, H = dock.geom_w, dock.geom_h
    ok_size = 150 <= W <= 660 and 90 <= H <= 360
    check("window size sane @ %s" % tag, ok_size, "%dx%d R=%.0f" % (W, H, dock.R))

    # every button inside window bounds
    in_win = all(0 <= b.cx - b.r and b.cx + b.r <= W and
                 0 <= b.cy - b.r and b.cy + b.r <= H for b in dock.buttons)
    check("all 10 buttons inside window @ %s" % tag, in_win)

    # main row ordering: files < settings < brave < terminal < appgrid (x)
    order = ["files", "settings", "brave", "terminal", "appgrid"]
    xs = [dock.by_key(k).cx for k in order]
    check("main row left->right order @ %s" % tag, xs == sorted(xs),
          str([round(x) for x in xs]))

    # hero biggest + centered
    hero = dock.by_key("brave")
    biggest = all(hero.r >= b.r for b in dock.buttons)
    check("hero button is largest @ %s" % tag, biggest,
          "r=%.1f" % hero.r)
    check("hero near center @ %s" % tag, abs(hero.cx - W / 2) < 2,
          "cx=%.1f W/2=%.1f" % (hero.cx, W / 2))

    # arc row above main row and no overlap with main circles
    arc = [dock.by_key(k) for k in
           ("bluetooth", "nightlight", "wifi", "power", "lock")]
    above = all(a.cy < dock.by_key("terminal").cy for a in arc)
    check("arc row sits above main row @ %s" % tag, above)
    overlap = False
    for a in arc:
        for k in ("files", "settings", "brave", "terminal", "appgrid"):
            m = dock.by_key(k)
            if math.hypot(a.cx - m.cx, a.cy - m.cy) < (a.r + m.r) * 0.8:
                overlap = True
    check("arc buttons do not overlap main buttons @ %s" % tag, not overlap)

    # input region: covers button centers...
    region = gdkwin.regions[-1][1] if gdkwin.regions else None
    check("input shape region created @ %s" % tag, region is not None)
    if region:
        covered = all(region.contains_point(int(b.cx), int(b.cy))
                      for b in dock.buttons)
        check("input region covers all buttons @ %s" % tag, covered)
        outside = region.contains_point(2, 2) or \
            region.contains_point(W - 3, 3) or \
            region.contains_point(3, 3)
        check("input region excludes corners (click-through) @ %s" % tag,
              not outside)

    # motion -> hover
    b = dock.by_key("settings")
    dock.on_motion(dock, MockEvent(b.cx, b.cy))
    check("motion sets hover @ %s" % tag, dock.hover_btn is b)
    dock.on_leave(dock, MockEvent(-1, -1))
    check("leave clears hover @ %s" % tag, dock.hover_btn is None)

    # press -> release -> action (PATH shims log commands)
    b = dock.by_key("files")
    dock.on_button_press(dock, MockEvent(b.cx, b.cy))
    press_ok = b.press
    dock.on_button_release(dock, MockEvent(b.cx, b.cy))
    time.sleep(0.2)
    press_ok = press_ok and not b.press
    check("press/release lifecycle @ %s" % tag, press_ok)
    check("Files button launches pcmanfm @ %s" % tag,
          "pcmanfm" in read_spawn_log(spawnlog), "")

    for key, cmd in (("settings", "bravos-settings"),
                     ("terminal", "lxterminal"),
                     ("lock", "bravos-lock")):
        b = dock.by_key(key)
        dock.on_button_press(dock, MockEvent(b.cx, b.cy))
        dock.on_button_release(dock, MockEvent(b.cx, b.cy))
    time.sleep(0.5)  # let the async PATH-shim processes flush
    log = read_spawn_log(spawnlog)
    check("Settings/terminal/lock buttons launch @ %s" % tag,
          all(c in log for c in
              ("bravos-settings", "lxterminal", "bravos-lock")))

    # render
    surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, W, H)
    cr = cairo.Context(surface)
    dock.on_draw(dock, cr)
    out = os.path.join(outpng, "dock-%dx%d.png" % (w, h))
    surface.write_to_png(out)
    check("rendered %s" % os.path.basename(out), os.path.exists(out))

    # power menu popup constructs and wires items
    pm = mod.PowerMenu(dock, 100, 100)
    check("power menu popup builds @ %s" % tag, True)

    # launcher popup builds
    la = mod.Launcher(dock)
    check("launcher popup builds @ %s" % tag, True)
    return dock


def main():
    outpng = sys.argv[1] if len(sys.argv) > 1 else "/tmp/shots"
    spawnlog = sys.argv[2] if len(sys.argv) > 2 else "/tmp/spawn.log"
    os.makedirs(outpng, exist_ok=True)
    install_mock_gi()
    dock_path = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                             "..", "rootfs", "usr", "local", "bin",
                             "bravos-dock")
    mod = load_dock(dock_path)

    for w, h in ((800, 600), (1280, 800), (1920, 1080), (3840, 2160)):
        test_at_size(mod, w, h, spawnlog, outpng)

    print("[docktest] PASS=%d FAIL=%d" % (PASS, FAIL))
    return 1 if FAIL else 0


if __name__ == "__main__":
    sys.exit(main())
