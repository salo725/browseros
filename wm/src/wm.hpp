// BraveWM - wm.hpp: the window manager core.
// A modern stacking WM for BraveOS: EWMH-compliant, workspaces, edge snapping,
// smooth animations, custom decorations, keyboard shortcuts.
#pragma once

#include "client.hpp"
#include "ewmh.hpp"

namespace bwm {

// signal flags (defined in main.cpp): SIGUSR1 = restart, SIGTERM/SIGINT = exit
extern volatile sig_atomic_t g_sigterm_flag;
extern volatile sig_atomic_t g_sigrestart_flag;

// EWMH helpers (implemented in wm.cpp)
void set_net_wm_state(Wm *wm, Client *c);
void set_net_allowed_actions(Wm *wm, Client *c);

class Menu;
class Switcher;
class Bar;

enum class Interact { Idle, Move, Resize };

struct MonitorRect {
    Rect rect;
    int id = 0;
    bool is_primary = true;
};

struct Strut {
    // reserved edges, in root coords
    int left = 0, right = 0, top = 0, bottom = 0;
    int left_start = 0, left_end = 0, right_start = 0, right_end = 0;
    int top_start = 0, top_end = 0, bottom_start = 0, bottom_end = 0;
};

class Wm {
   public:
    Display *dpy = nullptr;
    int screen = 0;
    Window root = None;
    Visual *visual = nullptr;
    Colormap colormap = None;
    unsigned int depth = 24;

    Atoms atoms;
    Config cfg;
    bool have_shape = false;
    bool have_xinerama = false;
    int shape_event_base = 0;

    Cursor cursor_normal = None, cursor_move = None;
    Cursor cursor_nw = None, cursor_ne = None, cursor_sw = None, cursor_se = None;
    Cursor cursor_h = None, cursor_v = None;

    std::vector<Client *> clients;    // all managed, manage order
    std::vector<Client *> stacking;   // bottom to top
    Client *focused_client = nullptr;
    std::vector<Client *> focus_mru;  // most-recently-used first

    int num_desktops = 2;
    int current_desktop = 0;
    bool showing_desktop = false;

    std::vector<MonitorRect> monitors;
    std::map<Window, Strut> struts;

    Window wm_check_win = None;

    // interactive state
    Interact interact = Interact::Idle;
    Client *interact_c = nullptr;
    int interact_edge = 0;
    int start_px = 0, start_py = 0;
    Rect start_geom;
    Window snap_preview = None;
    int snap_zone = 0;  // 0 none, 1 left, 2 right, 3 top(maximize), 4..7 corners
    Menu *menu = nullptr;
    Switcher *switcher = nullptr;
    Bar *bar = nullptr;

    Time last_event_time = CurrentTime;
    bool running = true;
    bool wm_error = false;

    // ---- lifecycle ----------------------------------------------------------
    bool init(const std::string &config_path);
    void run();
    void shutdown();

    // ---- client management --------------------------------------------------
    void scan_existing();
    void manage(Window w);
    void unmanage(Client *c, bool client_gone = false);
    Client *find_client(Window w) const;        // by client window
    Client *find_client_by_frame(Window f) const;
    void close(Client *c);                       // polite WM_DELETE
    void kill(Client *c);
    void iconify(Client *c);
    void uniconify(Client *c);
    void toggle_maximize(Client *c);
    void set_fullscreen(Client *c, bool on);
    void toggle_above(Client *c);
    void move_to_desktop(Client *c, int d);
    void move_window(Client *c, int x, int y);
    void resize_window(Client *c, int x, int y, int w, int h);
    void send_to_back(Client *c);
    void bring_to_front(Client *c);
    void restack_all();

    // ---- focus / desktops -----------------------------------------------------
    void focus(Client *c, bool raise_win = true);
    void unfocus();
    void update_focus_after(Client *gone);
    void switch_desktop(int d);
    void set_desktop_count(int n);
    void toggle_show_desktop();
    void set_show_desktop(bool on);
    void apply_desktop_visibility(bool animate);

    // ---- EWMH ------------------------------------------------------------------
    void sync_net_client_list();
    void set_active_window(Client *c);
    void update_desktop_props();
    void update_workarea();
    void read_struts(Client *c);
    Rect workarea_for(Client *c) const;
    Rect workarea_for_monitor(const Rect &mon) const;
    Rect monitor_for_point(int x, int y) const;
    Rect monitor_for_window(Client *c) const;
    void refresh_monitors();

    // ---- interactions ----------------------------------------------------------
    void start_interactive_move(Client *c, int px, int py);
    void start_interactive_resize(Client *c, int px, int py, int edge);
    void end_interactive();
    void update_snap_zone(int px, int py);
    Rect snap_target_rect(int zone, const Rect &wa) const;
    void show_snap_preview(Rect r);
    void hide_snap_preview();
    void draw_snap_preview();
    int snap_preview_w() const;
    int snap_preview_h() const;
    void snap_to_zone(Client *c);

    // ---- input --------------------------------------------------------------------
    void grab_keys();
    void on_key_press(XKeyEvent &e);
    void on_button_press(XButtonEvent &e);
    void on_button_release(XButtonEvent &e);
    void on_motion(XMotionEvent &e);
    void on_enter(XCrossingEvent &e);

    // ---- X event dispatch ------------------------------------------------------------
    void on_map_request(XMapRequestEvent &e);
    void on_configure_request(XConfigureRequestEvent &e);
    void on_destroy(XDestroyWindowEvent &e);
    void on_unmap(XUnmapEvent &e);
    void on_reparent(XReparentEvent &e);
    void on_property(XPropertyEvent &e);
    void on_client_message(XClientMessageEvent &e);
    void on_expose(XExposeEvent &e);

    // ---- helpers ------------------------------------------------------------------
    void spawn(const std::string &cmd) const;
    void open_root_menu(int x, int y);
    void open_window_menu(Client *c, int x, int y);
    void open_power_menu(int x, int y);
    void draw_client(Client *c);
    void tick_animations();
    void session_bar_tick();  // bar clock/status at ~1 Hz

    // keyboard layout cycling (delegated to bravos-kb-switch helper)
    void cycle_keyboard_layout();
};

// Paths / commands
extern const char *const KB_SWITCH_CMD;

}  // namespace bwm
