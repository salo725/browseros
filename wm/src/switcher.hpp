// BraveWM - switcher.hpp/.cpp: Alt+Tab application switcher overlay.
// A centered glass panel listing window titles with app icons; commit on
// Alt release (or timeout), cycle with Tab / Shift+Tab.
#pragma once

#include "wm.hpp"

namespace bwm {

class Switcher {
   public:
    Switcher(Wm *wm);
    ~Switcher();

   public:
    Window win = None;  // exposed for event routing

    void begin(bool backward);   // Alt+Tab pressed
    void step(bool backward);    // Tab pressed while active
    void commit();               // activate selection and close
    void cancel();
    bool active() const { return win != None; }
    void handle_key(XKeyEvent &e);
    void draw();

   private:
    void layout();

    Wm *wm;
    std::vector<Client *> list;
    int selected = 0;
    int x = 0, y = 0, w = 0, h = 0;
    double started = 0;
};

}  // namespace bwm
