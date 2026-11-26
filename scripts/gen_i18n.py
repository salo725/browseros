#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""gen_i18n.py — generate the BraveOS i18n conf files.

Reads the translation tables (i18n_shell.py, i18n_apps.py, i18n_install.py)
and writes, for every language and every app namespace:

    <out>/i18n/<app>/<lang>.conf     (key=value, UTF-8)

Aliased keys (dock tooltips that mirror WM labels, etc.) are resolved from
their source tables. The `time_24h` flag is written into the wm namespace.

usage: gen_i18n.py [outdir]     (default: <repo>/rootfs/usr/share/bravos)
"""
import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from i18n_shell import (DOCK, LOCK, SHELL_ALIASES, TIME_24H, WM)  # noqa: E402
from i18n_apps import SETTINGS  # noqa: E402
from i18n_install import INSTALLER, WELCOME  # noqa: E402

APPS = {
    "wm": WM,
    "dock": DOCK,
    "lock": LOCK,
    "settings": SETTINGS,
    "installer": INSTALLER,
    "welcome": WELCOME,
}

# gather all languages from all tables
LANGS = set()
for tbl in APPS.values():
    for key, langs in tbl.items():
        if isinstance(langs, dict):
            LANGS.update(langs.keys())
LANGS = sorted(LANGS)


def resolve(app, key):
    """Resolve a key's translations, following alias definitions."""
    seen = set()
    while True:
        if (app, key) in SHELL_ALIASES and (app, key) not in seen:
            seen.add((app, key))
            app, key = SHELL_ALIASES[(app, key)]
            continue
        break
    tbl = APPS.get(app, {})
    entry = tbl.get(key)
    return entry if isinstance(entry, dict) else None


def main():
    outdir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        REPO, "rootfs", "usr", "share", "bravos")
    i18n_dir = os.path.join(outdir, "i18n")

    written = 0
    for app, tbl in APPS.items():
        appdir = os.path.join(i18n_dir, app)
        os.makedirs(appdir, exist_ok=True)
        for lang in LANGS:
            lines = ["# BraveOS i18n — %s (%s)" % (app, lang)]
            for key in sorted(tbl.keys()):
                entry = resolve(app, key)
                text = None
                if entry:
                    text = entry.get(lang)
                if text is None:
                    continue  # English fallback lives in the app itself
                text = text.replace("\n", "\\n").replace("=", "\\=")
                lines.append("%s=%s" % (key, text))
            # wm-only: clock format flag
            if app == "wm" and lang in TIME_24H:
                lines.append("time_24h=1")
            path = os.path.join(appdir, lang + ".conf")
            with open(path, "w", encoding="utf-8") as f:
                f.write("\n".join(lines) + "\n")
            written += 1
    print("gen_i18n: wrote %d files for %d languages x %d apps"
          % (written, len(LANGS), len(APPS)))


if __name__ == "__main__":
    main()
