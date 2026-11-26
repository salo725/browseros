// BraveWM - bar.hpp: the BraveOS top status bar.
// Translucent dark glass bar: BraveOS brand + workspace pills on the left;
// Wi-Fi / Bluetooth / Battery / clock / power on the right. Dock-type window
// with struts, Pango text, i18n, responsive scaling, live status polling.
#pragma once

#include "wm.hpp"

namespace bwm {

class Bar {
   public:
    explicit Bar(Wm *wm);
    ~Bar();

    bool create();      // create the window (call once at WM init)
    void destroy_win();
    void relayout();    // recompute geometry/zones (resolution/monitor change)
    void draw();
    void refresh();     // full redraw (cheap; called on state changes)
    void tick();        // 1 Hz: clock, battery/wifi/bt polling, compositor check

    // event routing (return true if consumed)
    bool handle_button(XButtonEvent &e);
    bool handle_motion(XMotionEvent &e);
    bool handle_expose(XExposeEvent &e);
    bool handle_leave(XCrossingEvent &e);
    bool owns(Window w) const { return w == win; }

    int height() const { return h; }
    void open_power_menu(int px, int py);
    void logout();  // end the session cleanly

   private:
    // interactive zones (hit-testing for pills/icons)
    enum ZoneId {
        Z_NONE = 0,
        Z_PILL0 = 10,     // Z_PILL0+i -> workspace i (0-based)
        Z_ADD = 200,      // "+" pill
        Z_WIFI = 300,
        Z_BT = 301,
        Z_BATTERY = 302,
        Z_CLOCK = 303,
        Z_POWER = 304,
    };
    struct Zone {
        Rect r;
        int id;
    };

    // status snapshot
    struct BarStatus {
        bool battery_present = false;
        int battery_pct = 0;
        bool charging = false;
        bool wifi_present = false;
        bool wifi_on = false;   // radio enabled
        bool wifi_up = false;   // associated
        int wifi_quality = 0;   // 0..3
        bool bt_present = false;
        bool bt_on = false;
    };

    void poll_status();
    void poll_keyboard_layout();
    std::string clock_text();
    void layout_zones();
    int workspace_windows(int d) const;
    void recreate_window(bool argb);
    bool compositor_running() const;
    bool argb_visual_available();
    void ensure_window_mode();
    int zone_at(int px, int py) const;

    Wm *wm;
    Window win = None;
    int x = 0, y = 0, w = 0, h = 32;
    double u = 1.0;  // scale unit (h / 32)

    bool argb_mode = false;   // current window is 32-bit ARGB
    bool want_argb = false;   // compositor present + visual available
    bool have_argb = false;
    Visual *argb_visual = nullptr;
    Colormap argb_colormap = None;
    unsigned int argb_depth = 32;

    std::vector<Zone> zones;
    int hover_id = Z_NONE;
    int press_id = Z_NONE;
    double divider_x = 0;
    double brand_x = 0;
    bool time_24h = false;

    BarStatus status;
    std::string last_status_sig;

    time_t clock_sec = 0;
    int clock_shown_min = -1;
    std::string kb_layout;  // "" = default (us), else e.g. "ar"
};

}  // namespace bwm
