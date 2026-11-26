#!/bin/bash
# BraveOS v2 UI functional test (runs INSIDE the live guest).
# Exercises the real desktop over X: processes, dock clicks (ALL buttons),
# app grid, keyboard shortcuts, taskbar dynamics, power menu, lock, language
# switch. Prints UITEST-PASS/UITEST-FAIL lines; exits 0 only if all critical
# checks pass.
#
# Dock geometry (must mirror bshell dock_buttons(): 480x172 window centered
# at the bottom, dome circle center 240px below the window top, inner ring
# radius 170, outer ring radius 235):
#   Files     (SW/2-105, SH- 66)   inner, -38deg
#   Settings  (SW/2- 55, SH- 93)   inner, -19deg
#   Brave     (SW/2   , SH-102)   inner,    0deg (hero, 64px)
#   Terminal  (SW/2+ 55, SH- 93)   inner, +19deg
#   Grid      (SW/2+105, SH- 66)   inner, +38deg
#   Moon      (SW/2-131, SH-127)   outer, -34deg
#   Wifi      (SW/2- 69, SH-157)   outer, -17deg
#   Power     (SW/2+ 69, SH-157)   outer, +17deg
#   Lock      (SW/2+131, SH-127)   outer, +34deg
set -u
export DISPLAY=:0
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
PASS=0; FAIL=0
free -b | awk 'NR==2{printf "IDLERAM %s %s", $2, $7}'
ok()   { echo "UITEST-PASS $1"; PASS=$((PASS+1)); }
bad()  { echo "UITEST-FAIL $1 $2"; FAIL=$((FAIL+1)); }
shot() { /usr/local/bin/bshot "/tmp/uitest-$1.png" >/dev/null 2>&1; }
click(){ xdotool mousemove "$1" "$2" click 1; sleep "${3:-1.8}"; }
# diagnostic: which window is under the pointer, and what happened to it
diag(){ # diag <label> <x> <y>
  xdotool mousemove "$2" "$3" >/dev/null 2>&1
  W=$(xdotool getmouselocation --shell 2>/dev/null | sed -n 's/^WINDOW=//p')
  C=$(xprop -id "${W:-0}" WM_CLASS 2>/dev/null | cut -d= -f2)
  echo "UITEST-DIAG $1 ptr($2,$3) win=${W:-none} class=${C:-?}"
  xdotool mousemove "$2" "$3" click 1 >/dev/null 2>&1
}

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
nclients() { xprop -root _NET_CLIENT_LIST 2>/dev/null | grep -o '0x[0-9a-f]*' | wc -l; }
CL0=$(nclients)

# 3) open Settings with the Super+S shortcut (bwm global keybind)
xdotool key --clearmodifiers super+s; sleep 2.5
S=$(pgrep -c -x bsettings)
if [ "$S" -ge 1 ]; then ok "shortcut-super+s opens settings"; else bad "shortcut-super+s" "bsettings not running"; fi
shot settings

# 4) settings window is focused and named
ACTIVE=$(xdotool getactivewindow getwindowname 2>/dev/null | head -1)
case "$ACTIVE" in
  *Settings*) ok "settings-window-focused ($ACTIVE)" ;;
  *) bad "settings-window-focused" "active='$ACTIVE'" ;;
esac

# 5) close it again
pkill -x bsettings; sleep 0.8

# 6) screen geometry — xdotool getdisplaygeometry prints "W H" on ONE line.
#    (Do NOT use --shell here: it prints "WIDTH=..\nHEIGHT=..", which broke
#    every arithmetic expansion below in v2.0.0-rc.)
read -r SW SH <<< "$(xdotool getdisplaygeometry)"
if [ -n "${SW:-}" ] && [ -n "${SH:-}" ] && [ "$SW" -gt 0 ] && [ "$SH" -gt 0 ]; then
  ok "display-geometry ${SW}x${SH}"
else
  bad "display-geometry" "xdotool returned '${SW:-} ${SH:-}'"
fi

# 7) dock: Files button (left of the hero) launches the file manager
click $(( SW / 2 - 105 )) $(( SH - 66 ))
FM=$(pgrep -c -x pcmanfm)
[ "$FM" -ge 1 ] && ok "dock-files-launches-pcmanfm" || bad "dock-files" "no pcmanfm"
pkill -x pcmanfm; sleep 0.5

# 8) dock: app-grid button opens the on-screen grid (not the hidden one)
GRID=$(xdotool search --class bshell-grid 2>/dev/null | head -1)
diag grid $(( SW / 2 + 105 )) $(( SH - 66 )); sleep 1.8
sleep 0.6
grid_on_screen() {
  [ -n "${1:-}" ] || return 1
  xdotool getwindowgeometry --shell "$1" 2>/dev/null | \
    awk -F= '/^X=/{x=$2} END{exit !(x>=0)}'
}
if [ -n "${GRID:-}" ] && grid_on_screen "$GRID"; then
  ok "dock app-grid opens on screen"
else
  echo "UITEST-DIAG grid-geom-1: $(xdotool getwindowgeometry "$GRID" 2>&1 | tr '\n' ' ')"
  # second click (state could have been left open by a stray earlier press)
  diag grid $(( SW / 2 + 105 )) $(( SH - 66 )); sleep 1.5
  if ! grid_on_screen "$GRID"; then
    echo "UITEST-DIAG grid-geom-2: $(xdotool getwindowgeometry "$GRID" 2>&1 | tr '\n' ' ')"
    # probe the move path directly: if this works the click never fired,
    # if not the ConfigureRequest forwarding path is broken
    xdotool windowmove "$GRID" $(( (SW - 620) / 2 )) $(( (SH - 430) / 2 - 40 )) 2>&1 | sed 's/^/UITEST-DIAG windowmove: /'
    sleep 0.8
    echo "UITEST-DIAG grid-geom-3: $(xdotool getwindowgeometry "$GRID" 2>&1 | tr '\n' ' ')"
    bad "dock app-grid" "grid window not visible"
  else
    ok "dock app-grid opens on screen (second click)"
  fi
fi
shot grid
xdotool key --clearmodifiers Escape; sleep 0.5

# 9) dock: terminal button (right of the hero) launches xterm
click $(( SW / 2 + 55 )) $(( SH - 93 )) 2.5
XT=$(pgrep -c -x xterm)
if [ "$XT" -lt 1 ]; then
  echo "UITEST-INFO retry: xterm=$(ls -la /usr/bin/xterm 2>&1 | head -1)"
  click $(( SW / 2 + 55 )) $(( SH - 93 )) 3
  XT=$(pgrep -c -x xterm)
fi
[ "$XT" -ge 1 ] && ok "dock terminal launches xterm" || bad "dock terminal" "no xterm"
shot terminal

# 10) taskbar dynamics: the xterm window joins the client list
CL1=$(nclients)
if [ "$CL1" -gt "$CL0" ]; then
  ok "taskbar-tracks-new-window ($CL0 -> $CL1)"
else
  bad "taskbar-tracks-new-window" "client-list did not grow ($CL0 -> $CL1)"
fi
shot tasks
pkill -x xterm; sleep 0.5

# 11) dock: hero Brave button launches the browser
click $(( SW / 2 )) $(( SH - 102 )) 12
BR=$(pgrep -c -x brave)
[ "$BR" -ge 1 ] && ok "dock-hero-launches-brave" || bad "dock-hero" "no brave process"
shot brave-hero
pkill -x brave; sleep 1.5

# 12) dock: outer arc — Wi-Fi button opens Settings on the network page
click $(( SW / 2 - 69 )) $(( SH - 157 ))
S2=$(pgrep -c -x bsettings)
[ "$S2" -ge 1 ] && ok "dock-wifi-opens-settings" || bad "dock-wifi" "bsettings not running"
pkill -x bsettings; sleep 0.5

# 13) dock: outer arc — Moon toggles dark mode in the shell config
mkdir -p ~/.config/bravos
printf '{"language":"auto","dark":true}' > ~/.config/bravos/config.json
diag moon $(( SW / 2 - 131 )) $(( SH - 127 )); sleep 1.8
if grep -qE '"dark": ?false' ~/.config/bravos/config.json 2>/dev/null; then
  ok "dock-moon-toggles-dark"
else
  bad "dock-moon" "config still dark:true"
fi

# 14) dock: outer arc — Power button opens the power menu popup
diag power $(( SW / 2 + 69 )) $(( SH - 157 )); sleep 1.8
sleep 0.6
PWIN=$(xdotool search --class bshell-power 2>/dev/null | head -1)
if [ -n "${PWIN:-}" ] && grid_on_screen "$PWIN"; then
  ok "dock-power-opens-menu"
else
  bad "dock-power" "power menu not visible"
fi
shot power
xdotool key --clearmodifiers Escape; sleep 0.5

# 15) dock: outer arc — Lock button runs the screen locker (killed for the
#     sake of the remaining checks)
diag lock $(( SW / 2 + 131 )) $(( SH - 127 )); sleep 1.2
LK=$(pgrep -c -x xtrlock)
if [ "$LK" -ge 1 ]; then ok "dock-lock-runs-xtrlock"; else bad "dock-lock" "no xtrlock"; fi
pkill -x xtrlock; sleep 0.5

# 16) language switch: write German config, restart shell, verify redraw works
printf '{"language":"de"}' > ~/.config/bravos/config.json
touch /tmp/.bravos-config-stamp; sleep 1.5
pkill -x bshell; sleep 0.5; bshell >/dev/null 2>&1 & sleep 2
BS=$(pgrep -c -x bshell)
[ "$BS" -ge 1 ] && ok "language-switch-restart" || bad "language-switch-restart" "shell dead"
shot lang-de
printf '{"language":"auto"}' > ~/.config/bravos/config.json
touch /tmp/.bravos-config-stamp

# 17) RAM bookkeeping + idle-RAM breakdown
free -b | awk 'NR==2{printf "MEMB %s %s", $2, $7}'
echo "UITEST-MEM free-m:"
free -m | sed 's/^/UITEST-MEM /'
echo "UITEST-MEM meminfo:"
grep -E '^(MemTotal|MemFree|MemAvailable|Buffers|Cached|SwapCached|Shmem|Slab|SReclaimable|SUnreclaim|KernelStack|PageTables|AnonPages|Mapped|SwapTotal|SwapFree)' /proc/meminfo | sed 's/^/UITEST-MEM /'
echo "UITEST-MEM swap:"
swapon --show 2>/dev/null | sed 's/^/UITEST-MEM /'
echo "UITEST-MEM zram0 mm_stat:"
cat /sys/block/zram0/mm_stat 2>/dev/null | sed 's/^/UITEST-MEM /'
echo "UITEST-MEM tmpfs/overlay:"
df -h -t tmpfs -t overlay 2>/dev/null | sed 's/^/UITEST-MEM /'
echo "UITEST-MEM overlay dirs:"
du -sh /run/live/overlay* /overlayfs /var/cache/fontconfig 2>/dev/null | sed 's/^/UITEST-MEM /'

for sh in grid moon power lock; do
  if [ -f "/tmp/uitest-$sh.png" ]; then
    echo "UITEST-SHOT-BEGIN $sh"
    base64 -w0 "/tmp/uitest-$sh.png" 2>/dev/null
    echo ""
    echo "UITEST-SHOT-END $sh"
  fi
done

echo "UITEST-DONE pass=$PASS fail=$FAIL"
[ "$FAIL" -eq 0 ]
