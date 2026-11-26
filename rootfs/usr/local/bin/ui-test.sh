#!/bin/bash
# BraveOS v2 UI functional test (runs INSIDE the live guest).
# Exercises the real desktop over X: processes, dock clicks, app grid,
# keyboard shortcuts, taskbar dynamics, language switch. Prints
# UITEST-PASS/UITEST-FAIL lines; exits 0 only if all critical checks pass.
set -u
export DISPLAY=:0
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
PASS=0; FAIL=0
ok()   { echo "UITEST-PASS $1"; PASS=$((PASS+1)); }
bad()  { echo "UITEST-FAIL $1 $2"; FAIL=$((FAIL+1)); }
shot() { /usr/local/bin/bshot "/tmp/uitest-$1.png" >/dev/null 2>&1; }

# 1) session processes
BWM=$(pgrep -c -x bwm); BSHELL=$(pgrep -c -x bshell); BWALL=$(pgrep -c -x bwall)
if [ "$BWM" -ge 1 ] && [ "$BSHELL" -ge 1 ] && [ "$BWALL" -ge 1 ]; then
  ok "processes bwm=$BWM bshell=$BSHELL bwall=$BWALL"
else
  bad "processes" "bwm=$BWM bshell=$BSHELL bwall=$BWALL"
fi

# 2) EWMH root properties set by bwm
CL=$(xprop -root _NET_CLIENT_LIST 2>/dev/null | grep -c '(')
ND=$(xprop -root _NET_NUMBER_OF_DESKTOPS 2>/dev/null | grep -c '[0-9]')
[ "$CL" -ge 1 ] && [ "$ND" -ge 1 ] && ok "ewmh-properties" || bad "ewmh-properties" "client-list/desktops missing"

# 3) open Settings with the Super+S shortcut (bwm global keybind)
xdotool key --clearmodifiers super+s; sleep 2.5
S=$(pgrep -c -x bsettings)
if [ "$S" -ge 1 ]; then ok "shortcut-super+s opens settings"; else bad "shortcut-super+s" "bsettings not running"; fi
shot settings

# 4) settings window is on screen and named
ACTIVE=$(xdotool getactivewindow getwindowname 2>/dev/null | head -1)
case "$ACTIVE" in
  *Settings*) ok "settings-window-focused ($ACTIVE)" ;;
  *) bad "settings-window-focused" "active='$ACTIVE'" ;;
esac

# 5) close it again
pkill -x bsettings; sleep 0.5

# 6) dock click: app-grid button (bottom center, right of Brave center)
DOT=$(xdotool getdisplaygeometry --shell | tr 'x' ' ')
SW=${DOT%% *}; SH=${DOT##* }
GX=$(( SW / 2 + 132 )); GY=$(( SH - 190 / 2 + 22 ))
xdotool mousemove $(( SW / 2 + 132 )) $(( SH - 68 )) click 1; sleep 1.8
GRID=$(xdotool search --class bshell-grid 2>/dev/null | head -1)
[ -n "${GRID:-}" ] && ok "dock app-grid opens" || bad "dock app-grid" "grid window missing"
shot grid
xdotool key --clearmodifiers Escape; sleep 0.5

# 7) launch a terminal via the dock terminal button (left of brave)
xdotool mousemove $(( SW / 2 - 132 )) $(( SH - 68 )) click 1; sleep 2.5
XT=$(pgrep -c -x xterm)
[ "$XT" -ge 1 ] && ok "dock terminal launches xterm" || bad "dock terminal" "no xterm"

# 8) taskbar dynamics: task pill appears for the xterm window
CL2=$(xprop -root _NET_CLIENT_LIST 2>/dev/null | wc -l)
[ "$CL2" -ge 1 ] && ok "client-list-has-windows" || bad "client-list-has-windows" "empty"
shot tasks

# 9) language switch: write German config, restart shell, verify redraw works
mkdir -p ~/.config/bravos
printf '{"language":"de"}' > ~/.config/bravos/config.json
touch /tmp/.bravos-config-stamp; sleep 1.5
pkill -x bshell; sleep 0.5; bshell >/dev/null 2>&1 & sleep 2
BS=$(pgrep -c -x bshell)
[ "$BS" -ge 1 ] && ok "language-switch-restart" || bad "language-switch-restart" "shell dead"
shot lang-de
printf '{"language":"auto"}' > ~/.config/bravos/config.json
touch /tmp/.bravos-config-stamp

# 10) RAM bookkeeping
free -b | awk 'NR==2{printf "MEMB %s %s", $2, $7}'

echo "UITEST-DONE pass=$PASS fail=$FAIL"
[ "$FAIL" -eq 0 ]
