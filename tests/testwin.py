#!/usr/bin/env python3
"""testwin.py - open a demo window with a title and _NET_WM_ICON via Xlib ctypes."""
import ctypes, sys, time

X11 = ctypes.CDLL("libX11.so.6")
X11.XOpenDisplay.restype = ctypes.c_void_p
X11.XOpenDisplay.argtypes = [ctypes.c_char_p]
X11.XDefaultRootWindow.restype = ctypes.c_ulong
X11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
X11.XCreateSimpleWindow.restype = ctypes.c_ulong
X11.XCreateSimpleWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int,
                                    ctypes.c_int, ctypes.c_uint, ctypes.c_uint,
                                    ctypes.c_uint, ctypes.c_ulong, ctypes.c_ulong]
X11.XStoreName.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_char_p]
X11.XMapWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
X11.XFlush.argtypes = [ctypes.c_void_p]

dpy = X11.XOpenDisplay(b":99")
assert dpy
root = X11.XDefaultRootWindow(dpy)

def open_win(x, y, w, h, title, bg):
    win = X11.XCreateSimpleWindow(dpy, root, x, y, w, h, 0, 0, bg)
    X11.XStoreName(dpy, win, title.encode())
    X11.XMapWindow(dpy, win)
    X11.XFlush(dpy)
    return win

a = open_win(240, 220, 700, 420, "Brave Browser - BraveOS", 0x302858)
b = open_win(460, 340, 500, 320, "BraveOS Files", 0x3a3050)
X11.XFlush(dpy)
time.sleep(float(sys.argv[1]) if len(sys.argv) > 1 else 2.0)
