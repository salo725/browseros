#!/bin/bash
# start-session.sh - persistent Xvfb + BraveWM test session for local dev.
SIZE="${1:-1280x800}"
pkill -f "Xvfb :99" 2>/dev/null
pkill -x bravos-wm 2>/dev/null
sleep 0.3
export DISPLAY=:99
Xvfb :99 -screen 0 "${SIZE}x24" -nolisten tcp &
sleep 1.2
cd "$(dirname "$0")/../wm"
setsid ./bravos-wm 2>/tmp/bravos-wm.log &
sleep 1.0
echo "session up: display=:99 size=$SIZE"
