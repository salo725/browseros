#!/usr/bin/env python3
"""Fix sign-compare warnings in BraveWM sources."""
import os

WM = "/home/z/my-project/browseros/wm/src"

def patch(fname, old, new):
    path = os.path.join(WM, fname)
    content = open(path).read()
    if old in content:
        open(path, "w").write(content.replace(old, new))
        print(f"patched {fname}")
    else:
        print(f"pattern not found in {fname} (maybe already patched)")

patch("wm.cpp",
      "std::vector<long> types =\n        get_cardinals(wm->dpy, w, wm->atoms.net_wm_window_type, 8);\n    for (long t : types) {",
      "std::vector<long> types =\n        get_cardinals(wm->dpy, w, wm->atoms.net_wm_window_type, 8);\n    for (unsigned long t : types) {")

for f in ("wm.cpp", "events.cpp", "switcher.cpp"):
    patch(f, "c->desktop != 0xFFFFFFFF", "c->desktop != (int)0xFFFFFFFF")
    patch(f, "(c->desktop == 0xFFFFFFFF", "(c->desktop == (int)0xFFFFFFFF")

patch("wm.cpp", "if (c->desktop == 0xFFFFFFFF)", "if (c->desktop == (int)0xFFFFFFFF)")
patch("wm.cpp", "if (c->desktop != 0xFFFFFFFF &&", "if (c->desktop != (int)0xFFFFFFFF &&")
patch("events.cpp", "bool show = (c->desktop == 0xFFFFFFFF", "bool show = (c->desktop == (int)0xFFFFFFFF")
patch("wm.cpp", "bool show = (c->desktop == 0xFFFFFFFF", "bool show = (c->desktop == (int)0xFFFFFFFF")
patch("wm.cpp", "showable = (c->desktop == 0xFFFFFFFF", "showable = (c->desktop == (int)0xFFFFFFFF")
patch("events.cpp", "showable = (c->desktop == 0xFFFFFFFF", "showable = (c->desktop == (int)0xFFFFFFFF")
patch("events.cpp", "if (a->desktop != 0xFFFFFFFF && a->desktop != current_desktop)",
      "if (a->desktop != (int)0xFFFFFFFF && a->desktop != current_desktop)")
patch("events.cpp", "if (c->desktop == 0xFFFFFFFF) {", "if (c->desktop == (int)0xFFFFFFFF) {")
