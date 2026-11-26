// BraveWM - client.hpp: managed client window + custom frame decorations.
#pragma once

#include "ewmh.hpp"

namespace bwm {

struct Wm;

enum class WinType {
    Normal, Desktop, Dock, Toolbar, Menu, Utility, Splash, Dialog,
    Dropdown, Popup, Tooltip, Notification, Combo
};

// Which titlebar button the pointer is over (hit test result).
enum BtnHit { BTN_NONE = 0, BTN_CLOSE, BTN_MAX, BTN_MIN, BTN_MENU };

struct Config {
    int titlebar_h = 28;
    int border_w = 1;
    int corner_r = 10;
    std::string font = "Inter";
    Color c_bg_focused{0x1b / 255.0, 0x14 / 255.0, 0x33 / 255.0};
    Color c_bg_unfocused{0x12 / 255.0, 0x0e / 255.0, 0x20 / 255.0};
    Color c_border_focused = Color::from_hex("#6c3ce9");
    Color c_border_unfocused = Color::from_hex("#2a2450");
    Color c_text_focused = Color::from_hex("#ffffff");
    Color c_text_unfocused = Color::from_hex("#9a94b8");
    Color c_accent = Color::from_hex("#f04e23");
    Color c_accent2 = Color::from_hex("#6c3ce9");
    Color c_btn_min = Color::from_hex("#8b87a8");
    int workspaces = 2;
    bool focus_follows_mouse = false;
    bool animations = true;
    int anim_steps = 5;
    int anim_step_ms = 28;
    std::string term_cmd = "lxterminal";
    std::string files_cmd = "pcmanfm";
    std::string browser_cmd = "brave";
    std::string settings_cmd = "bravos-settings";
    std::string launcher_cmd = "bravos-launcher";
    std::string lock_cmd = "bravos-lock";
    std::string kb_layouts = "us,ar";
};

struct Client {
    Wm *wm = nullptr;
    Window win = None;
    Window frame = None;
    std::string title;
    std::string res_name, res_class;
    WinType type = WinType::Normal;
    int desktop = 0;          // -1 = all desktops (sticky)
    bool iconified = false;
    bool maximized = false;   // both vert+horz
    bool fullscreen = false;
    bool above = false;
    bool urgent = false;
    bool wants_decor = true;
    bool never_focus = false; // input hints say do not focus
    bool visible = false;     // currently mapped on the current desktop
    bool iconic_by_wm = false; // we initiated the unmap (iconify)
    bool hidden_by_show_desktop = false;
    Window transient_for = None;

    Rect geom;     // client geometry relative to root (current, incl. frame offset)
    Rect restore;  // geometry before maximize/fullscreen
    Rect frame_restore;

    // _NET_WM_ICON (best small icon), stored as ARGB rows
    std::vector<unsigned int> icon_data;
    int icon_w = 0, icon_h = 0;

    int hover_btn = BTN_NONE;
    bool pressed_btn = false;

    long last_click_ms = 0;   // double-click detection
    int edge = 0;             // interactive resize edge bitmask

    double anim_t0 = 0;       // animation: start time
    Rect anim_from, anim_to;
    int anim_phase = 0;       // 0 = map/maximize open, 1 = minimize close
    bool animating = false;

    // frame geometry helpers (frame = border + titlebar + client)
    Rect frame_rect() const;
    Rect titlebar_rect() const;   // relative to frame
    Rect button_rect(int which) const;  // which: 1=close 2=max 3=min, rel. frame
    const Config &cf() const;

    BtnHit hit_button(int fx, int fy) const;   // frame-relative coords
    int hit_edge(int fx, int fy) const;        // frame-relative resize edges
};

// Frame + decoration implementation -------------------------------------------

void client_frame_create(Client *c);
void client_frame_destroy(Client *c);
void client_frame_configure(Client *c);         // sync frame size/pos to geom
void client_draw_decorations(Client *c);
void client_apply_shape(Client *c);
void client_read_icon(Client *c);
void client_set_wm_state(Client *c, int state);
void client_sync_frame_extents(Client *c);
void client_anim_start(Client *c, const Rect &to, bool closing);
void client_anim_step(Client *c, double now);

}  // namespace bwm
