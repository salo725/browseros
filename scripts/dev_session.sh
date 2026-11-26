#!/usr/bin/env bash
# Local full-stack test of BraveOS v2 shell under Xvfb.
# Usage: bash /home/z/my-project/braveos/scripts/dev_session.sh
set -u
REPO=/home/z/my-project/braveos
BIN=$REPO/rust/target/release
ASSETS=$REPO/build/assets
export DISPLAY=:95
export BRAVOS_ICONS_DIR=$ASSETS
export BRAVOS_WALLPAPER_DIR=$ASSETS
export BRAVOS_I18N_DIR=$REPO/rust/i18n
export PATH=$BIN:$PATH

pkill -f "Xvfb :95" 2>/dev/null || true
pkill -f "target/release/bwm" 2>/dev/null || true
pkill -f "target/release/bwall" 2>/dev/null || true
pkill -f "target/release/bshell" 2>/dev/null || true
pkill -f "target/release/bsettings" 2>/dev/null || true
sleep 0.3
Xvfb :95 -screen 0 1280x800x24 &
sleep 1

mkdir -p /home/z/my-project/braveos/build/logs
LOG=/home/z/my-project/braveos/build/logs
$BIN/bwm > $LOG/bwm.log 2>&1 &
sleep 0.7
$BIN/bwall > $LOG/bwall.log 2>&1 &
$BIN/bshell > $LOG/bshell.log 2>&1 &
sleep 1.2
echo "session up; pids:"
pgrep -a -f "$BIN/(bwm|bwall|bshell)" | head
