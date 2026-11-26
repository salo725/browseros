// BraveWM - i18n.hpp: WM string internationalization.
// Loads key=value translations from /usr/share/bravos/i18n/wm/<lang>.conf
// (UTF-8, generated from the shared BraveOS translation table). Falls back
// to the compiled-in English table, then to the key itself.
#pragma once

#include <map>
#include <string>

namespace bwm {

class I18n {
   public:
    // Resolve language: BRAVEOS_LANG > LANG > "en"; strips encoding/country
    // and tries "<ll_CC>" then "<ll>".
    void init();

    // Translate a key; fallback chain: loaded table -> English table -> key.
    const std::string &tr(const char *key) const;
    bool has(const char *key) const;
    const std::string &lang() const { return lang_; }
    bool rtl() const { return rtl_; }

   private:
    void load_file(const std::string &path);
    std::map<std::string, std::string> table_;
    std::string lang_ = "en";
    bool rtl_ = false;
    // storage for fallback chains (tr() must return a stable ref)
    mutable std::string last_resort_;
};

extern I18n g_i18n;

}  // namespace bwm
