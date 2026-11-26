// BraveWM - menu.hpp: custom popup menus (desktop right-click menu, window
// menu). Cairo-drawn, dark glass, orange selection highlight.
#pragma once

#include "wm.hpp"

#include <functional>

namespace bwm {

struct MenuEntry {
    std::string label;
    std::string icon;                 // reserved for icons
    std::function<void()> callback;   // null = separator/disabled
    bool enabled = true;
};

class Menu {
   public:
    Menu(Wm *wm);
    ~Menu();

   public:
    Window win = None;  // exposed for event routing

    void show(std::vector<MenuEntry> items, int x, int y, bool near_pointer);
    void hide();
    bool active() const { return win != None; }
    void handle_button(XButtonEvent &e);
    void handle_motion(XMotionEvent &e);
    void handle_key(XKeyEvent &e);
    void handle_expose(XExposeEvent &e);

   private:
    void draw();
    int item_at(int y) const;
    Rect item_rect(int i) const;

    Wm *wm;
    std::vector<MenuEntry> items;
    int hover = -1;
    int x = 0, y = 0, w = 0, h = 0;
};

}  // namespace bwm
