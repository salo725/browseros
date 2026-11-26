// BraveWM - i18n.cpp: WM string internationalization (see i18n.hpp).
#include "i18n.hpp"

#include <clocale>
#include <cstdlib>
#include <fstream>
#include <map>

namespace bwm {

I18n g_i18n;

// compiled-in English fallback (the single source of truth for keys)
static const std::map<std::string, std::string> kEnglish = {
    {"menu_browser", "Brave Browser"},
    {"menu_files", "Files"},
    {"menu_terminal", "Terminal"},
    {"menu_settings", "Settings"},
    {"menu_appgrid", "App Grid"},
    {"menu_shortcuts", "Keyboard Shortcuts"},
    {"menu_lock", "Lock Screen"},
    {"menu_restart_wm", "Restart BraveWM"},
    {"menu_maximize", "Maximize"},
    {"menu_unmaximize", "Unmaximize"},
    {"menu_minimize", "Minimize"},
    {"menu_fullscreen", "Fullscreen"},
    {"menu_exit_fullscreen", "Exit Fullscreen"},
    {"menu_always_top", "Always on Top"},
    {"menu_unpin", "Unpin (Always on Top off)"},
    {"menu_move_ws", "Move to Workspace %d"},
    {"menu_close", "Close"},
    {"power_lock", "Lock Screen"},
    {"power_logout", "Log Out"},
    {"power_suspend", "Suspend"},
    {"power_restart", "Restart"},
    {"power_shutdown", "Shut Down"},
    {"am", "AM"},
    {"pm", "PM"},
    {"ws", "WS"},
};

static std::string trim(const std::string &s) {
    size_t a = s.find_first_not_of(" \t\r\n");
    if (a == std::string::npos) return "";
    size_t b = s.find_last_not_of(" \t\r\n");
    return s.substr(a, b - a + 1);
}

static std::string strip_quotes(const std::string &s) {
    if (s.size() >= 2 && ((s.front() == '"' && s.back() == '"') ||
                          (s.front() == '\'' && s.back() == '\'')))
        return s.substr(1, s.size() - 2);
    return s;
}

void I18n::load_file(const std::string &path) {
    std::ifstream f(path);
    if (!f) return;
    std::string line;
    while (std::getline(f, line)) {
        std::string s = trim(line);
        if (s.empty() || s[0] == '#') continue;
        size_t eq = s.find('=');
        if (eq == std::string::npos) continue;
        std::string k = trim(s.substr(0, eq));
        std::string v = strip_quotes(trim(s.substr(eq + 1)));
        if (!k.empty() && !v.empty()) table_[k] = v;
    }
}

void I18n::init() {
    const char *env = getenv("BRAVEOS_LANG");
    std::string ll = env ? env : "";
    if (ll.empty()) {
        env = getenv("LANG");
        ll = env ? env : "";
    }
    // strip encoding ("ar_SA.UTF-8" -> "ar_SA")
    size_t dot = ll.find('.');
    if (dot != std::string::npos) ll = ll.substr(0, dot);
    if (!ll.empty() && ll != "C" && ll != "POSIX") {
        load_file("/usr/share/bravos/i18n/wm/" + ll + ".conf");
        std::string base = ll.substr(0, 2);
        if (base != ll) load_file("/usr/share/bravos/i18n/wm/" + base + ".conf");
        lang_ = table_.empty() ? base : base;
    }
    rtl_ = (lang_ == "ar" || lang_ == "he" || lang_ == "fa" || lang_ == "ur");
}

bool I18n::has(const char *key) const {
    return table_.count(key) > 0 || kEnglish.count(key) > 0;
}

const std::string &I18n::tr(const char *key) const {
    auto it = table_.find(key);
    if (it != table_.end()) return it->second;
    auto e = kEnglish.find(key);
    if (e != kEnglish.end()) return e->second;
    last_resort_ = key;
    return last_resort_;
}

}  // namespace bwm
