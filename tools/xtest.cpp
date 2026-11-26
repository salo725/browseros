// xtest - BraveWM functional test client (used by CI and local testing).
// Verifies reparenting, EWMH properties, desktop switching, maximize,
// close protocol - everything that can be checked without synthetic input.
// usage: xtest <display>
#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/Xatom.h>

#include <cairo/cairo.h>

#include <cstdio>
#include <cstring>
#include <string>
#include <unistd.h>

static int g_fail = 0;
static void check(bool ok, const char *name, const char *detail = "") {
    printf("[xtest] %-40s %s %s\n", name, ok ? "PASS" : "FAIL", detail);
    if (!ok) g_fail++;
}

static Window wait_mapped(Display *dpy, Window w, int tries = 80) {
    for (int i = 0; i < tries; i++) {
        XWindowAttributes a;
        if (XGetWindowAttributes(dpy, w, &a) && a.map_state == IsViewable)
            return w;
        usleep(50000);
    }
    return None;
}

static Atom atom_(Display *d, const char *n) { return XInternAtom(d, n, False); }

static Window get_window_prop(Display *d, Window root, Atom prop) {
    Atom type;
    int fmt;
    unsigned long n, after;
    unsigned char *data = nullptr;
    Window out = None;
    if (XGetWindowProperty(d, root, prop, 0, 1, False, XA_WINDOW, &type, &fmt, &n,
                          &after, &data) == Success &&
        data && n == 1) {
        out = *(Window *)data;
    }
    if (data) XFree(data);
    return out;
}

int main(int argc, char **argv) {
    const char *dname = argc > 1 ? argv[1] : ":0";
    Display *dpy = XOpenDisplay(dname);
    if (!dpy) {
        printf("[xtest] FAIL cannot open display\n");
        return 1;
    }
    Window root = DefaultRootWindow(dpy);
    check(True, "open-display", dname);

    // ---- WM detection ------------------------------------------------------
    Window wmcheck = get_window_prop(dpy, root, atom_(dpy, "_NET_SUPPORTING_WM_CHECK"));
    check(wmcheck != None, "net-supporting-wm-check-set");
    std::string wmname;
    {
        Atom type;
        int fmt;
        unsigned long n, after;
        unsigned char *data = nullptr;
        if (XGetWindowProperty(dpy, wmcheck, atom_(dpy, "_NET_WM_NAME"), 0, 256,
                               False, AnyPropertyType, &type, &fmt, &n, &after,
                               &data) == Success &&
            data) {
            wmname = (const char *)data;
            XFree(data);
        }
    }
    check(wmname == "BraveWM", "wm-name-is-bravos-wm", wmname.c_str());

    long ndesk = 0;
    {
        Atom type;
        int fmt;
        unsigned long n, after;
        unsigned char *data = nullptr;
        if (XGetWindowProperty(dpy, root, atom_(dpy, "_NET_NUMBER_OF_DESKTOPS"), 0,
                               1, False, XA_CARDINAL, &type, &fmt, &n, &after,
                               &data) == Success &&
            data) {
            ndesk = *(long *)data;
            XFree(data);
        }
    }
    check(ndesk >= 1, "net-number-of-desktops", std::to_string(ndesk).c_str());

    // ---- map a window, verify management ------------------------------------
    XSetWindowAttributes wa = {};
    wa.event_mask = StructureNotifyMask | PropertyChangeMask;
    wa.background_pixel = WhitePixel(dpy, 0);
    Window w1 = XCreateWindow(dpy, root, 60, 60, 420, 300, 0, CopyFromParent,
                              InputOutput, CopyFromParent, CWBackPixel | CWEventMask, &wa);
    XClassHint ch = {(char *)"xtest", (char *)"XTest"};
    XSetClassHint(dpy, w1, &ch);
    XStoreName(dpy, w1, "BraveOS Test Window");
    XWMHints hints = {};
    hints.flags = InputHint | StateHint;
    hints.input = True;
    hints.initial_state = NormalState;
    XSetWMHints(dpy, w1, &hints);
    Atom protocols = atom_(dpy, "WM_DELETE_WINDOW");
    XSetWMProtocols(dpy, w1, &protocols, 1);
    XMapWindow(dpy, w1);
    check(wait_mapped(dpy, w1) != None, "window-mapped");

    usleep(300000);  // let the WM manage it

    // reparented? parent should be the WM frame, not root
    Window rret, parent, *children;
    unsigned int nch;
    XQueryTree(dpy, w1, &rret, &parent, &children, &nch);
    if (children) XFree(children);
    check(parent != root, "window-reparented-into-frame");

    // _NET_CLIENT_LIST
    bool in_list = false;
    {
        Atom type;
        int fmt;
        unsigned long n, after;
        unsigned char *data = nullptr;
        if (XGetWindowProperty(dpy, root, atom_(dpy, "_NET_CLIENT_LIST"), 0, 64,
                               False, XA_WINDOW, &type, &fmt, &n, &after,
                               &data) == Success &&
            data) {
            Window *wins = (Window *)data;
            for (unsigned long i = 0; i < n; i++)
                if (wins[i] == w1) in_list = true;
            XFree(data);
        }
    }
    check(in_list, "net-client-list-contains-window");

    // active window should be our window (focused on map)
    Window active = get_window_prop(dpy, root, atom_(dpy, "_NET_ACTIVE_WINDOW"));
    check(active == w1, "net-active-window-on-map");

    // frame extents (decorations)
    {
        Atom type;
        int fmt;
        unsigned long n, after;
        unsigned char *data = nullptr;
        long ext[4] = {0, 0, 0, 0};
        bool got = false;
        if (XGetWindowProperty(dpy, w1, atom_(dpy, "_NET_FRAME_EXTENTS"), 0, 4,
                               False, XA_CARDINAL, &type, &fmt, &n, &after,
                               &data) == Success &&
            data && n == 4) {
            memcpy(ext, data, sizeof(ext));
            got = true;
            XFree(data);
        }
        check(got && ext[2] > 0, "net-frame-extents",
              (std::to_string(ext[0]) + "," + std::to_string(ext[1]) + "," +
               std::to_string(ext[2]) + "," + std::to_string(ext[3]))
                  .c_str());
    }

    // ---- EWMH maximize via client message (what panel/taskbar does) ---------
    {
        XEvent e = {};
        e.xclient.type = ClientMessage;
        e.xclient.window = w1;
        e.xclient.message_type = atom_(dpy, "_NET_WM_STATE");
        e.xclient.format = 32;
        e.xclient.data.l[0] = 1;  // _NET_WM_STATE_ADD
        e.xclient.data.l[1] = (long)atom_(dpy, "_NET_WM_STATE_MAXIMIZED_VERT");
        e.xclient.data.l[2] = (long)atom_(dpy, "_NET_WM_STATE_MAXIMIZED_HORZ");
        XSendEvent(dpy, root, False, SubstructureRedirectMask | SubstructureNotifyMask, &e);
        XFlush(dpy);
        usleep(400000);  // allow animation to finish
        XWindowAttributes a;
        XGetWindowAttributes(dpy, w1, &a);
        bool maxed = a.width >= DisplayWidth(dpy, 0) - 80 &&
                    a.height >= DisplayHeight(dpy, 0) - 80;
        check(maxed, "ewmh-maximize-window",
              (std::to_string(a.width) + "x" + std::to_string(a.height)).c_str());
        // restore
        e.xclient.data.l[0] = 0;  // _NET_WM_STATE_REMOVE
        XSendEvent(dpy, root, False, SubstructureRedirectMask | SubstructureNotifyMask, &e);
        XFlush(dpy);
        usleep(200000);
    }

    // ---- desktop switching via root property (what panel pills do) ----------
    if (ndesk >= 2) {
        long one = 1;
        XChangeProperty(dpy, root, atom_(dpy, "_NET_CURRENT_DESKTOP"), XA_CARDINAL,
                        32, PropModeReplace, (unsigned char *)&one, 1);
        XFlush(dpy);
        usleep(300000);
        XWindowAttributes a;
        XGetWindowAttributes(dpy, w1, &a);
        bool hidden = a.map_state != IsViewable;
        check(hidden, "ewmh-switch-desktop-hides-window");
        long zero = 0;
        XChangeProperty(dpy, root, atom_(dpy, "_NET_CURRENT_DESKTOP"), XA_CARDINAL,
                        32, PropModeReplace, (unsigned char *)&zero, 1);
        XFlush(dpy);
        usleep(300000);
        XGetWindowAttributes(dpy, w1, &a);
        check(a.map_state == IsViewable, "ewmh-switch-back-shows-window");
    }

    // ---- number of desktops change (panel "+" button) ------------------------
    {
        long four = 4;
        XChangeProperty(dpy, root, atom_(dpy, "_NET_NUMBER_OF_DESKTOPS"), XA_CARDINAL,
                        32, PropModeReplace, (unsigned char *)&four, 1);
        XFlush(dpy);
        usleep(200000);
        long got = 0;
        Atom type;
        int fmt;
        unsigned long n, after;
        unsigned char *data = nullptr;
        if (XGetWindowProperty(dpy, root, atom_(dpy, "_NET_NUMBER_OF_DESKTOPS"), 0, 1,
                               False, XA_CARDINAL, &type, &fmt, &n, &after,
                               &data) == Success &&
            data) {
            got = *(long *)data;
            XFree(data);
        }
        check(got == 4, "panel-add-desktops-via-property");
        // back to 2
        long two = 2;
        XChangeProperty(dpy, root, atom_(dpy, "_NET_NUMBER_OF_DESKTOPS"), XA_CARDINAL,
                        32, PropModeReplace, (unsigned char *)&two, 1);
        XFlush(dpy);
        usleep(200000);
    }

    // ---- close via _NET_CLOSE_WINDOW (what dock/taskbar does) -----------------
    {
        bool closed = false;
        // wait for the WM to send WM_DELETE_WINDOW; we destroy in response
        XEvent e = {};
        e.xclient.type = ClientMessage;
        e.xclient.window = w1;
        e.xclient.message_type = atom_(dpy, "_NET_CLOSE_WINDOW");
        e.xclient.format = 32;
        XSendEvent(dpy, root, False, SubstructureRedirectMask | SubstructureNotifyMask, &e);
        XFlush(dpy);
        for (int i = 0; i < 60; i++) {
            while (XPending(dpy)) {
                XNextEvent(dpy, &e);
                if (e.type == ClientMessage &&
                    e.xclient.message_type == atom_(dpy, "WM_PROTOCOLS") &&
                    (Atom)e.xclient.data.l[0] == atom_(dpy, "WM_DELETE_WINDOW")) {
                    closed = true;
                }
                if (e.type == DestroyNotify && e.xdestroywindow.window == w1)
                    closed = true;
            }
            if (closed) break;
            usleep(50000);
        }
        check(closed, "net-close-window-protocol");
        XDestroyWindow(dpy, w1);
        XFlush(dpy);
        usleep(200000);
    }

    // ---- a second round: two windows for stacking sanity ------------------------
    XSetWindowAttributes wa2 = {};
    wa2.event_mask = StructureNotifyMask;
    wa2.background_pixel = WhitePixel(dpy, 0);
    Window a = XCreateWindow(dpy, root, 100, 100, 300, 200, 0, CopyFromParent,
                             InputOutput, CopyFromParent, CWBackPixel | CWEventMask, &wa2);
    Window b = XCreateWindow(dpy, root, 200, 160, 300, 200, 0, CopyFromParent,
                             InputOutput, CopyFromParent, CWBackPixel | CWEventMask, &wa2);
    XStoreName(dpy, a, "A");
    XStoreName(dpy, b, "B");
    XMapWindow(dpy, a);
    XMapWindow(dpy, b);
    wait_mapped(dpy, a);
    wait_mapped(dpy, b);
    usleep(300000);
    Window active2 = get_window_prop(dpy, root, atom_(dpy, "_NET_ACTIVE_WINDOW"));
    check(active2 == b, "second-window-takes-focus");
    XDestroyWindow(dpy, a);
    XDestroyWindow(dpy, b);
    XFlush(dpy);

    XCloseDisplay(dpy);
    printf("[xtest] %s (%d failures)\n", g_fail ? "FAILED" : "ALL OK", g_fail);
    return g_fail ? 1 : 0;
}
