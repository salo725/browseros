// BraveWM - ewmh.hpp: EWMH/ICCCM atom registry and property helpers.
// Implements enough of the EWMH 1.4 spec for panels, docks and toolkits
// (workspace switching, taskbar lists, close/maximize requests, struts).
#pragma once

#include "util.hpp"

namespace bwm {

struct Atoms {
    // EWMH root + client atoms
    Atom net_supported;
    Atom net_supporting_wm_check;
    Atom net_client_list;
    Atom net_client_list_stacking;
    Atom net_number_of_desktops;
    Atom net_desktop_geometry;
    Atom net_desktop_viewport;
    Atom net_current_desktop;
    Atom net_desktop_names;
    Atom net_active_window;
    Atom net_workarea;
    Atom net_showing_desktop;
    Atom net_close_window;
    Atom net_wm_moveresize;
    Atom net_restack_window;
    Atom net_wm_name;
    Atom net_wm_visible_name;
    Atom net_wm_icon;
    Atom net_wm_state;
    Atom net_wm_state_modal;
    Atom net_wm_state_sticky;
    Atom net_wm_state_maximized_vert;
    Atom net_wm_state_maximized_horz;
    Atom net_wm_state_fullscreen;
    Atom net_wm_state_above;
    Atom net_wm_state_below;
    Atom net_wm_state_demands_attention;
    Atom net_wm_state_add;
    Atom net_wm_state_remove;
    Atom net_wm_state_toggle;
    Atom net_wm_window_type;
    Atom net_wm_window_type_desktop;
    Atom net_wm_window_type_dock;
    Atom net_wm_window_type_toolbar;
    Atom net_wm_window_type_menu;
    Atom net_wm_window_type_utility;
    Atom net_wm_window_type_splash;
    Atom net_wm_window_type_dialog;
    Atom net_wm_window_type_dropdown;
    Atom net_wm_window_type_popup;
    Atom net_wm_window_type_tooltip;
    Atom net_wm_window_type_notification;
    Atom net_wm_window_type_combo;
    Atom net_wm_window_type_normal;
    Atom net_wm_allowed_actions;
    Atom net_wm_action_move;
    Atom net_wm_action_resize;
    Atom net_wm_action_minimize;
    Atom net_wm_action_maximize_horz;
    Atom net_wm_action_maximize_vert;
    Atom net_wm_action_close;
    Atom net_wm_action_fullscreen;
    Atom net_wm_action_change_desktop;
    Atom net_wm_action_stick;
    Atom net_wm_desktop;
    Atom net_wm_pid;
    Atom net_wm_strut;
    Atom net_wm_strut_partial;
    Atom net_wm_icon_geometry;
    Atom net_frame_extents;
    // ICCCM atoms
    Atom wm_state;
    Atom wm_protocols;
    Atom wm_delete_window;
    Atom wm_take_focus;
    Atom wm_change_state;
    Atom wm_normal_hints;
    Atom wm_name;
    Atom wm_icon_name;
    Atom wm_class;
    Atom wm_transient_for;
    Atom wm_hint_flags;
    Atom utf8_string;
    Atom wm_window_role;
    Atom manager;
    Atom multiple;
    Atom targets;
    Atom atom_pair;
    Atom cardinal;

    void init(Display *dpy) {
        auto a = [&](const char *n) { return XInternAtom(dpy, n, False); };
        net_supported = a("_NET_SUPPORTED");
        net_supporting_wm_check = a("_NET_SUPPORTING_WM_CHECK");
        net_client_list = a("_NET_CLIENT_LIST");
        net_client_list_stacking = a("_NET_CLIENT_LIST_STACKING");
        net_number_of_desktops = a("_NET_NUMBER_OF_DESKTOPS");
        net_desktop_geometry = a("_NET_DESKTOP_GEOMETRY");
        net_desktop_viewport = a("_NET_DESKTOP_VIEWPORT");
        net_current_desktop = a("_NET_CURRENT_DESKTOP");
        net_desktop_names = a("_NET_DESKTOP_NAMES");
        net_active_window = a("_NET_ACTIVE_WINDOW");
        net_workarea = a("_NET_WORKAREA");
        net_showing_desktop = a("_NET_SHOWING_DESKTOP");
        net_close_window = a("_NET_CLOSE_WINDOW");
        net_wm_moveresize = a("_NET_WM_MOVERESIZE");
        net_restack_window = a("_NET_RESTACK_WINDOW");
        net_wm_name = a("_NET_WM_NAME");
        net_wm_visible_name = a("_NET_WM_VISIBLE_NAME");
        net_wm_icon = a("_NET_WM_ICON");
        net_wm_state = a("_NET_WM_STATE");
        net_wm_state_modal = a("_NET_WM_STATE_MODAL");
        net_wm_state_sticky = a("_NET_WM_STATE_STICKY");
        net_wm_state_maximized_vert = a("_NET_WM_STATE_MAXIMIZED_VERT");
        net_wm_state_maximized_horz = a("_NET_WM_STATE_MAXIMIZED_HORZ");
        net_wm_state_fullscreen = a("_NET_WM_STATE_FULLSCREEN");
        net_wm_state_above = a("_NET_WM_STATE_ABOVE");
        net_wm_state_below = a("_NET_WM_STATE_BELOW");
        net_wm_state_demands_attention = a("_NET_WM_STATE_DEMANDS_ATTENTION");
        net_wm_state_add = a("_NET_WM_STATE_ADD");
        net_wm_state_remove = a("_NET_WM_STATE_REMOVE");
        net_wm_state_toggle = a("_NET_WM_STATE_TOGGLE");
        net_wm_window_type = a("_NET_WM_WINDOW_TYPE");
        net_wm_window_type_desktop = a("_NET_WM_WINDOW_TYPE_DESKTOP");
        net_wm_window_type_dock = a("_NET_WM_WINDOW_TYPE_DOCK");
        net_wm_window_type_toolbar = a("_NET_WM_WINDOW_TYPE_TOOLBAR");
        net_wm_window_type_menu = a("_NET_WM_WINDOW_TYPE_MENU");
        net_wm_window_type_utility = a("_NET_WM_WINDOW_TYPE_UTILITY");
        net_wm_window_type_splash = a("_NET_WM_WINDOW_TYPE_SPLASH");
        net_wm_window_type_dialog = a("_NET_WM_WINDOW_TYPE_DIALOG");
        net_wm_window_type_dropdown = a("_NET_WM_WINDOW_TYPE_DROPDOWN_MENU");
        net_wm_window_type_popup = a("_NET_WM_WINDOW_TYPE_POPUP_MENU");
        net_wm_window_type_tooltip = a("_NET_WM_WINDOW_TYPE_TOOLTIP");
        net_wm_window_type_notification = a("_NET_WM_WINDOW_TYPE_NOTIFICATION");
        net_wm_window_type_combo = a("_NET_WM_WINDOW_TYPE_COMBO");
        net_wm_window_type_normal = a("_NET_WM_WINDOW_TYPE_NORMAL");
        net_wm_allowed_actions = a("_NET_WM_ALLOWED_ACTIONS");
        net_wm_action_move = a("_NET_WM_ACTION_MOVE");
        net_wm_action_resize = a("_NET_WM_ACTION_RESIZE");
        net_wm_action_minimize = a("_NET_WM_ACTION_MINIMIZE");
        net_wm_action_maximize_horz = a("_NET_WM_ACTION_MAXIMIZE_HORZ");
        net_wm_action_maximize_vert = a("_NET_WM_ACTION_MAXIMIZE_VERT");
        net_wm_action_close = a("_NET_WM_ACTION_CLOSE");
        net_wm_action_fullscreen = a("_NET_WM_ACTION_FULLSCREEN");
        net_wm_action_change_desktop = a("_NET_WM_ACTION_CHANGE_DESKTOP");
        net_wm_action_stick = a("_NET_WM_ACTION_STICK");
        net_wm_desktop = a("_NET_WM_DESKTOP");
        net_wm_pid = a("_NET_WM_PID");
        net_wm_strut = a("_NET_WM_STRUT");
        net_wm_strut_partial = a("_NET_WM_STRUT_PARTIAL");
        net_wm_icon_geometry = a("_NET_WM_ICON_GEOMETRY");
        net_frame_extents = a("_NET_FRAME_EXTENTS");
        wm_state = a("WM_STATE");
        wm_protocols = a("WM_PROTOCOLS");
        wm_delete_window = a("WM_DELETE_WINDOW");
        wm_take_focus = a("WM_TAKE_FOCUS");
        wm_change_state = a("WM_CHANGE_STATE");
        wm_normal_hints = XA_WM_NORMAL_HINTS;
        wm_name = XA_WM_NAME;
        wm_icon_name = XA_WM_ICON_NAME;
        wm_class = XA_WM_CLASS;
        utf8_string = a("UTF8_STRING");
        manager = a("MANAGER");
        cardinal = XA_CARDINAL;
    }

    // All atoms we advertise in _NET_SUPPORTED.
    std::vector<Atom> supported_list() const {
        return {
            net_supported,           net_supporting_wm_check,
            net_client_list,         net_client_list_stacking,
            net_number_of_desktops,  net_desktop_geometry,
            net_desktop_viewport,    net_current_desktop,
            net_desktop_names,       net_active_window,
            net_workarea,           net_showing_desktop,
            net_close_window,        net_wm_moveresize,
            net_restack_window,      net_wm_name,
            net_wm_visible_name,     net_wm_icon,
            net_wm_state,            net_wm_state_modal,
            net_wm_state_sticky,     net_wm_state_maximized_vert,
            net_wm_state_maximized_horz, net_wm_state_fullscreen,
            net_wm_state_above,      net_wm_state_below,
            net_wm_state_demands_attention, net_wm_allowed_actions,
            net_wm_action_move,      net_wm_action_resize,
            net_wm_action_minimize,  net_wm_action_maximize_horz,
            net_wm_action_maximize_vert, net_wm_action_close,
            net_wm_action_fullscreen, net_wm_action_change_desktop,
            net_wm_action_stick,     net_wm_desktop,
            net_wm_pid,              net_wm_strut_partial,
            net_frame_extents,
        };
    }
};

// ---- property helpers ------------------------------------------------------

inline bool get_cardinal(Display *dpy, Window w, Atom prop, long *out) {
    Atom type;
    int fmt;
    unsigned long nitems, bytes_after;
    unsigned char *data = nullptr;
    if (XGetWindowProperty(dpy, w, prop, 0, 1, False, XA_CARDINAL, &type, &fmt,
                           &nitems, &bytes_after, &data) != Success)
        return false;
    if (nitems < 1 || !data) {
        if (data) XFree(data);
        return false;
    }
    *out = *(long *)data;
    XFree(data);
    return true;
}

inline bool get_text(Display *dpy, Window w, Atom prop, std::string *out) {
    Atom type;
    int fmt;
    unsigned long nitems, bytes_after;
    unsigned char *data = nullptr;
    if (XGetWindowProperty(dpy, w, prop, 0, 8192, False, AnyPropertyType, &type,
                           &fmt, &nitems, &bytes_after, &data) != Success)
        return false;
    if (!data || nitems == 0) {
        if (data) XFree(data);
        return false;
    }
    if (type == XA_STRING || type == XInternAtom(dpy, "UTF8_STRING", False) ||
        type == XInternAtom(dpy, "COMPOUND_TEXT", False)) {
        *out = std::string((const char *)data, nitems);
    }
    XFree(data);
    return true;
}

inline std::vector<long> get_cardinals(Display *dpy, Window w, Atom prop,
                                       long max_items) {
    std::vector<long> out;
    Atom type;
    int fmt;
    unsigned long nitems, bytes_after;
    unsigned char *data = nullptr;
    if (XGetWindowProperty(dpy, w, prop, 0, max_items, False, XA_CARDINAL, &type,
                           &fmt, &nitems, &bytes_after, &data) != Success)
        return out;
    if (data) {
        long *vals = (long *)data;
        for (unsigned long i = 0; i < nitems; i++) out.push_back(vals[i]);
        XFree(data);
    }
    return out;
}

inline bool has_protocol(Display *dpy, Window w, Atom protocol) {
    Atom *protos;
    int n;
    bool found = false;
    if (XGetWMProtocols(dpy, w, &protos, &n)) {
        for (int i = 0; i < n; i++)
            if (protos[i] == protocol) found = true;
        XFree(protos);
    }
    return found;
}

}  // namespace bwm
