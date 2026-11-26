#!/usr/bin/env bash
# =============================================================================
# BraveOS - build.sh
# Builds a bootable live ISO (BIOS + UEFI) from a clean Debian minbase root.
#   1. debootstrap Debian bookworm (minbase)  -> clean slate, no distro layer
#   2. install kernel + firmware + X + openbox session + Brave + pcmanfm
#   3. overlay BraveOS custom UI (session, settings app, installer, theme)
#   4. squashfs (zstd) + grub-mkrescue hybrid ISO
# Must run as root (CI runs `sudo -E bash build.sh`).
# =============================================================================
set -Eeuo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="$(tr -d '[:space:]' < "$REPO_DIR/VERSION")"
DISTRO="bookworm"
MIRROR="http://deb.debian.org/debian"
BUILD_DIR="${BRAVOS_BUILD_DIR:-$REPO_DIR/build}"
ROOTFS="$BUILD_DIR/rootfs"
OUT="$BUILD_DIR/output"
ISO_TREE="$BUILD_DIR/isotree"
OVERLAY="$REPO_DIR/rootfs"

log()  { echo -e "[bravos-build] $*"; }
fail() { echo -e "[bravos-build][FAIL] $*" >&2; exit 1; }

[ "$(id -u)" -eq 0 ] || fail "run as root"
mkdir -p "$ROOTFS" "$OUT"

log "version: $VERSION"

# -----------------------------------------------------------------------------
# 0. Verify host tools
# -----------------------------------------------------------------------------
for t in debootstrap mksquashfs grub-mkrescue rsvg-convert curl python3; do
  command -v "$t" >/dev/null 2>&1 || fail "missing host tool: $t"
done

# Rust toolchain (CI installs it, but be self-sufficient otherwise)
if ! command -v cargo >/dev/null 2>&1; then
  if [ -x "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  elif [ -x /root/.cargo/bin/cargo ]; then
    export PATH="/root/.cargo/bin:$PATH"
  else
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain stable >/dev/null 2>&1 \
      || fail "no Rust toolchain and rustup install failed"
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
fi

# -----------------------------------------------------------------------------
# 1. Host-side asset generation (wallpapers, icons, grub background)
# -----------------------------------------------------------------------------
log "generating assets"
BRAVOS_DESIGN_PNG="${BRAVOS_DESIGN_PNG:-/home/z/my-project/upload/BraveOS.png}" \
  python3 "$REPO_DIR/scripts/gen_assets.py" "$BUILD_DIR/assets"
ASSETS="$BUILD_DIR/assets"
# canonical wallpaper name used by the shell/wallpaper tool
cp -f "$ASSETS/wallpaper-dark.png" "$ASSETS/braveos.png"
cp -f "$ASSETS/wallpaper-light.png" "$ASSETS/braveos-light.png"
cp -f "$ASSETS/wallpaper-aurora.png" "$ASSETS/braveos-aurora.png"

# -----------------------------------------------------------------------------
# 2. debootstrap - clean base
# -----------------------------------------------------------------------------
# older hosts may not know the bookworm suite; the generic 'gutsy' template
# works fine when an explicit mirror is given
if [ ! -e /usr/share/debootstrap/scripts/bookworm ] && [ -d /usr/share/debootstrap/scripts ]; then
  log "adding bookworm symlink to debootstrap scripts"
  ln -sf gutsy /usr/share/debootstrap/scripts/bookworm
fi

if [ ! -e "$ROOTFS/bin/sh" ]; then
  log "debootstrap $DISTRO (minbase)"
  debootstrap --variant=minbase --include="systemd-sysv,dbus,ca-certificates,sudo" \
    "$DISTRO" "$ROOTFS" "$MIRROR"
fi

# Mount pseudo-filesystems inside chroot
mount --bind /dev  "$ROOTFS/dev"  2>/dev/null || true
mount --bind /dev/pts "$ROOTFS/dev/pts" 2>/dev/null || true
mount -t proc proc "$ROOTFS/proc" 2>/dev/null || true
mount --bind /sys  "$ROOTFS/sys"  2>/dev/null || true
cleanup_mounts() {
  umount -l "$ROOTFS/dev/pts" 2>/dev/null || true
  umount -l "$ROOTFS/dev" 2>/dev/null || true
  umount -l "$ROOTFS/proc" 2>/dev/null || true
  umount -l "$ROOTFS/sys" 2>/dev/null || true
}
trap 'cleanup_mounts' EXIT

cp /etc/resolv.conf "$ROOTFS/etc/resolv.conf"

# Prevent services from starting inside the chroot
cat > "$ROOTFS/usr/sbin/policy-rc.d" <<'EOF'
#!/bin/sh
exit 101
EOF
chmod +x "$ROOTFS/usr/sbin/policy-rc.d"

# -----------------------------------------------------------------------------
# 3. apt sources: Debian + Brave official repository
# -----------------------------------------------------------------------------
log "configuring apt sources"
mkdir -p "$ROOTFS/etc/apt/keyrings"
cat > "$ROOTFS/etc/apt/sources.list" <<EOF
deb $MIRROR $DISTRO main contrib non-free non-free-firmware
deb $MIRROR $DISTRO-updates main contrib non-free non-free-firmware
deb http://security.debian.org/debian-security $DISTRO-security main contrib non-free non-free-firmware
EOF

curl -fsSL https://brave-browser-apt-release.s3.brave.com/brave-core.asc \
  -o "$ROOTFS/etc/apt/keyrings/brave-core.asc" || fail "cannot download Brave signing key"
cat > "$ROOTFS/etc/apt/sources.list.d/brave-browser-release.list" <<EOF
deb [signed-by=/etc/apt/keyrings/brave-core.asc] https://brave-browser-apt-release.s3.brave.com stable main
EOF

run_in_rootfs() { chroot "$ROOTFS" env DEBIAN_FRONTEND=noninteractive "$@"; }

# wipe any partial indexes debootstrap may have left, force a full update
rm -rf "$ROOTFS/var/lib/apt/lists/"*

log "apt update"
run_in_rootfs apt-get update

# -----------------------------------------------------------------------------
# 4. Install package set
# -----------------------------------------------------------------------------
log "installing packages"
PKGS="$(grep -v '^\s*#' "$REPO_DIR/config/packages.list" | grep -v '^\s*$' | tr '\n' ' ')"
run_in_rootfs apt-get install -y --no-install-recommends $PKGS \
  || fail "package installation failed - check package names"

# Optional X DDX drivers: install individually, tolerate absence.
for p in xserver-xorg-video-vesa xserver-xorg-video-nouveau xserver-xorg-video-vmware; do
  run_in_rootfs apt-get install -y --no-install-recommends "$p" || \
    log "optional driver package not available: $p"
done

run_in_rootfs apt-get -f install -y

# -----------------------------------------------------------------------------
# 5. Overlay BraveOS custom UI, config and apps
# -----------------------------------------------------------------------------
log "copying overlay files"
cp -a "$OVERLAY/." "$ROOTFS/"

# --- Rust UI stack (bwm, bshell, bsettings, bwall, bshot) ---
log "building Rust UI stack"
( cd "$REPO_DIR/rust" && cargo build --release --offline 2>/dev/null ) \
  || ( cd "$REPO_DIR/rust" && cargo build --release ) \
  || fail "cargo build failed"
for bin in bwm bshell bsettings bwall bshot; do
  cp "$REPO_DIR/rust/target/release/$bin" "$ROOTFS/usr/local/bin/$bin"
  chmod 755 "$ROOTFS/usr/local/bin/$bin"
done
mkdir -p "$ROOTFS/usr/share/bravos/i18n"
cp "$REPO_DIR"/rust/i18n/*.json "$ROOTFS/usr/share/bravos/i18n/"
log "rust stack installed ($(ls "$ROOTFS/usr/local/bin" | grep -c '^b' ) binaries)"

# Generated assets into rootfs
mkdir -p "$ROOTFS/usr/share/bravos/wallpapers" \
         "$ROOTFS/usr/share/bravos/icons" \
         "$ROOTFS/usr/share/bravos/grub" \
         "$ROOTFS/usr/share/icons/hicolor/48x48/apps" \
         "$ROOTFS/usr/share/backgrounds"
echo "$VERSION" > "$ROOTFS/usr/share/bravos/VERSION"
cp "$ASSETS/wallpaper-dark.png" "$ASSETS/wallpaper-light.png" \
   "$ASSETS/wallpaper-aurora.png" "$ROOTFS/usr/share/bravos/wallpapers/" 2>/dev/null || true
cp -f "$ASSETS/braveos.png" "$ROOTFS/usr/share/bravos/wallpapers/braveos.png" 2>/dev/null || true
cp -f "$ASSETS/braveos-light.png" "$ROOTFS/usr/share/bravos/wallpapers/braveos-light.png" 2>/dev/null || true
cp -f "$ASSETS/braveos-aurora.png" "$ROOTFS/usr/share/bravos/wallpapers/braveos-aurora.png" 2>/dev/null || true
cp -f "$ASSETS/wallpaper-dark.png" "$ROOTFS/usr/share/backgrounds/bravos-dark.png"
cp "$ASSETS"/icon-*.png "$ROOTFS/usr/share/bravos/icons/" 2>/dev/null || true
cp "$ASSETS"/icon-*.png "$ROOTFS/usr/share/icons/hicolor/48x48/apps/" 2>/dev/null || true
cp "$ASSETS"/icon-*.png "$ROOTFS/usr/local/share/icons/hicolor/48x48/apps/" 2>/dev/null || \
  { mkdir -p "$ROOTFS/usr/local/share/icons/hicolor/48x48/apps"; cp "$ASSETS"/icon-*.png "$ROOTFS/usr/local/share/icons/hicolor/48x48/apps/" 2>/dev/null || true; }
cp "$ASSETS"/grub-bg.png "$ROOTFS/usr/share/bravos/grub/grub-bg.png"

# Brave initial preferences (start page, sane defaults).
# The official deb installs to /opt/brave.com/brave; look there first, then
# fall back to parsing the launcher script for the real app directory.
BRAVE_DIR=""
for cand in "$ROOTFS/opt/brave.com/brave" "$ROOTFS/usr/lib/brave-browser"; do
  [ -d "$cand" ] && BRAVE_DIR="$cand" && break
done
if [ -z "$BRAVE_DIR" ] && [ -x "$ROOTFS/usr/bin/brave-browser-stable" ]; then
  APP="$(sed -n 's/^[^#]*exec[^"]*"\(\/[^"]*brave[^"]*\)".*/\1/p' \
    "$ROOTFS/usr/bin/brave-browser-stable" | head -n1)"
  [ -n "$APP" ] && [ -d "$ROOTFS$(dirname "$APP")" ] && \
    BRAVE_DIR="$ROOTFS$(dirname "$APP")"
fi
if [ -n "$BRAVE_DIR" ]; then
  cp "$REPO_DIR/config/initial_preferences" "$BRAVE_DIR/initial_preferences"
  cp "$REPO_DIR/config/initial_preferences" "$BRAVE_DIR/master_preferences"
  log "brave initial_preferences installed to $BRAVE_DIR"
else
  fail "brave app dir not found (checked /opt/brave.com/brave, /usr/lib/brave-browser)"
fi

# -----------------------------------------------------------------------------
# 6. System configuration inside rootfs
# -----------------------------------------------------------------------------
log "configuring system"
echo "braveos" > "$ROOTFS/etc/hostname"
cat > "$ROOTFS/etc/hosts" <<'EOF'
127.0.0.1 localhost
127.0.1.1 braveos
EOF

# Live user (uid 1000) - create base account first, then join groups that
# actually exist (some groups like plugdev may be absent in a minbase chroot)
chroot "$ROOTFS" groupadd -g 1000 brave 2>/dev/null || true
chroot "$ROOTFS" useradd -m -u 1000 -g 1000 -s /bin/bash brave \
  || chroot "$ROOTFS" usermod -u 1000 -g 1000 brave 2>/dev/null || true
for g in video audio input plugdev netdev adm disk cdrom; do
  chroot "$ROOTFS" groupadd -f "$g" 2>/dev/null || true
  chroot "$ROOTFS" usermod -aG "$g" brave 2>/dev/null || true
done
chroot "$ROOTFS" bash -c "echo 'brave:brave' | chpasswd"
chroot "$ROOTFS" bash -c "echo 'root:bravos-root' | chpasswd"
echo "brave ALL=(ALL:ALL) NOPASSWD:ALL" > "$ROOTFS/etc/sudoers.d/bravos"
chmod 0440 "$ROOTFS/etc/sudoers.d/bravos"
chroot "$ROOTFS" visudo -c

# polkit: live user may run the installer without password prompts
cat > "$ROOTFS/etc/polkit-1/rules.d/49-bravos.rules" <<'EOF'
polkit.addRule(function(action, subject) {
    if (subject.user == "brave") {
        if (action.id == "com.bravos.installer.run") {
            return polkit.Result.YES;
        }
        if (action.id.indexOf("org.freedesktop.NetworkManager.") == 0) {
            return polkit.Result.YES;
        }
    }
});
EOF

# Performance tuning: reserve resources for the browser
cat > "$ROOTFS/etc/sysctl.d/90-bravos.conf" <<'EOF'
# BraveOS: keep RAM free for the browser
vm.swappiness = 10
vm.vfs_cache_pressure = 50
vm.dirty_background_bytes = 16777216
vm.dirty_bytes = 50331648
EOF

# journald volatile (no persistent logs on disk)
mkdir -p "$ROOTFS/etc/systemd/journald.conf.d"
cat > "$ROOTFS/etc/systemd/journald.conf.d/bravos.conf" <<'EOF'
[Journal]
Storage=volatile
RuntimeMaxUse=16M
EOF

# zram: compressed RAM swap so small-RAM machines keep going
mkdir -p "$ROOTFS/etc/systemd/system/zramswap.service.d"
cat > "$ROOTFS/etc/default/zramswap" <<'EOF'
ALGO=zstd
PERCENT=50
EOF

# Enable session + tuning services; graphical target
chroot "$ROOTFS" systemctl enable bravos-session.service bravos-tuning.service 2>/dev/null || true
chroot "$ROOTFS" systemctl set-default graphical.target
chroot "$ROOTFS" systemctl disable getty@tty1.service 2>/dev/null || true

# xdg dirs
mkdir -p "$ROOTFS/home/brave/.config"
chroot "$ROOTFS" chown -R brave:brave /home/brave

# overlay files were copied by the CI user; normalize ownership to root.
# NOTE: /etc is NOT chowned wholesale - /etc/shadow must stay root:shadow.
for d in /etc/xdg /etc/systemd /etc/skel /etc/polkit-1 /etc/default \
         /etc/sysctl.d /etc/hostname /etc/hosts; do
  chroot "$ROOTFS" chown -R root:root "$d" 2>/dev/null || true
done
chroot "$ROOTFS" chown -R root:root /usr 2>/dev/null || true
chroot "$ROOTFS" chown root:shadow /etc/shadow /etc/gshadow 2>/dev/null || true
chroot "$ROOTFS" chown -R brave:brave /home/brave 2>/dev/null || true

# Empty machine-id: regenerated on each live boot
: > "$ROOTFS/etc/machine-id"
rm -f "$ROOTFS/var/lib/dbus/machine-id"

# Restore the canonical setuid set and verify: any filesystem op that drops
# the suid bit breaks sudo (and friends) in the guest. Fail the build loudly
# instead of shipping an image where privileged operations cannot work.
for b in /usr/bin/sudo /usr/bin/passwd /usr/bin/su /usr/bin/mount /usr/bin/umount \
         /usr/bin/chsh /usr/bin/chfn /usr/bin/newgrp /usr/bin/gpasswd; do
  [ -f "$ROOTFS$b" ] && chroot "$ROOTFS" chmod 4755 "$b"
done
if [ -f "$ROOTFS/usr/lib/dbus-1.0/dbus-daemon-launch-helper" ]; then
  chroot "$ROOTFS" chown root:messagebus /usr/lib/dbus-1.0/dbus-daemon-launch-helper
  chroot "$ROOTFS" chmod 4750 /usr/lib/dbus-1.0/dbus-daemon-launch-helper
fi
[ -f "$ROOTFS/usr/bin/sudo" ] || fail "sudo binary missing from rootfs"
[ -u "$ROOTFS/usr/bin/sudo" ] || fail "/usr/bin/sudo lost suid bit"
[ "$(stat -c %u "$ROOTFS/usr/bin/sudo")" = "0" ] || fail "/usr/bin/sudo not owned by root"
chroot "$ROOTFS" visudo -c >/dev/null || fail "sudoers validation failed"
log "suid set restored and verified"

# Rebuild initramfs WITH live-boot hooks
log "updating initramfs (live-boot hooks)"
chroot "$ROOTFS" update-initramfs -u -t

# -----------------------------------------------------------------------------
# 7. Cleanup rootfs (size matters)
# -----------------------------------------------------------------------------
log "cleaning rootfs"
run_in_rootfs apt-get clean
run_in_rootfs rm -rf /var/lib/apt/lists/* /var/cache/apt/archives/*.deb \
  /var/log/* /var/tmp/*
chroot "$ROOTFS" rm -rf /usr/share/doc/* /usr/share/man/* /usr/share/info/* \
  /usr/share/gnome/help 2>/dev/null || true
# strip non-en locales, keep C/C.UTF-8/en*
find "$ROOTFS/usr/share/locale" -mindepth 1 -maxdepth 1 -type d \
  ! -name 'C*' ! -name 'en*' -exec rm -rf {} + 2>/dev/null || true
rm -f "$ROOTFS/usr/sbin/policy-rc.d" "$ROOTFS/etc/resolv.conf"

# Pre-generate fontconfig caches AFTER cleanup: on a live boot, a missing
# cache makes fontconfig rebuild it into the tmpfs overlay (~200MB for
# Noto-CJK), which counted against the idle-RAM budget. Generated here, the
# caches live in the read-only squashfs (page cache only, reclaimable).
log "pre-generating fontconfig caches"
run_in_rootfs sh -c 'fc-cache -f >/dev/null 2>&1 || true'
[ -d "$ROOTFS/var/cache/fontconfig" ] && \
  log "fontconfig cache: $(du -sh "$ROOTFS/var/cache/fontconfig" | cut -f1)" || \
  log "WARNING: no fontconfig cache generated"

# -----------------------------------------------------------------------------
# 8. Squashfs
# -----------------------------------------------------------------------------
log "creating squashfs (zstd)"
mkdir -p "$ISO_TREE/live" "$ISO_TREE/boot/grub"
cleanup_mounts || true
# live-boot needs the mountpoint DIRECTORIES inside the squashfs (dev, proc,
# sys, run) - empty them but keep them, systemd mounts tmpfs over them.
rm -rf "$ROOTFS/run"/* "$ROOTFS/tmp"/* 2>/dev/null || true
mksquashfs "$ROOTFS" "$ISO_TREE/live/filesystem.squashfs" \
  -comp zstd -Xcompression-level 19 -noappend -no-recovery >/dev/null
log "squashfs size: $(du -h "$ISO_TREE/live/filesystem.squashfs" | cut -f1)"

# Kernel + initrd
VMLINUZ="$(ls "$ROOTFS/boot/vmlinuz-"* | sort -V | tail -n1)"
INITRD="$(ls "$ROOTFS/boot/initrd.img-"* | sort -V | tail -n1)"
cp "$VMLINUZ" "$ISO_TREE/live/vmlinuz"
cp "$INITRD"  "$ISO_TREE/live/initrd.img"
log "kernel: $(basename "$VMLINUZ")"

# Package manifest shipped on the ISO
chroot "$ROOTFS" dpkg-query -W -f='${Package}\t${Version}\n' | sort > "$ISO_TREE/bravos-packages.txt"

# -----------------------------------------------------------------------------
# 9. GRUB (BIOS + UEFI hybrid)
# -----------------------------------------------------------------------------
cp "$REPO_DIR/grub/grub.cfg" "$ISO_TREE/boot/grub/grub.cfg"
cp "$ASSETS/grub-bg.png" "$ISO_TREE/boot/grub/grub-bg.png"
mkdir -p "$ISO_TREE/boot/grub/fonts"
grub-mkfont -s 16 -o "$ISO_TREE/boot/grub/fonts/dejavu16.pf2" \
  /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf 2>/dev/null || true

# Standard medium marker: the GRUB EFI stub and live-boot tooling search for
# /.disk/info on the booted medium.
mkdir -p "$ISO_TREE/.disk"
echo "BraveOS $VERSION (amd64)" > "$ISO_TREE/.disk/info"

# isolinux entries so Rufus "ISO Image" mode (BIOS) can boot the extracted
# FAT volume exactly like our GRUB path does.
ISOLINUX_BIN=""
for p in /usr/lib/ISOLINUX/isolinux.bin /usr/lib/syslinux/modules/bios/isolinux.bin; do
  [ -f "$p" ] && ISOLINUX_BIN="$p" && break
done
if [ -n "$ISOLINUX_BIN" ]; then
  mkdir -p "$ISO_TREE/isolinux"
  cp "$ISOLINUX_BIN" "$ISO_TREE/isolinux/isolinux.bin"
  LDLINUX=""
  for p in /usr/lib/ISOLINUX/ldlinux.c32 /usr/lib/syslinux/modules/bios/ldlinux.c32; do
    [ -f "$p" ] && LDLINUX="$p" && break
  done
  [ -n "$LDLINUX" ] && cp "$LDLINUX" "$ISO_TREE/isolinux/ldlinux.c32"
  cat > "$ISO_TREE/isolinux/isolinux.cfg" <<'EOF'
SERIAL 0 115200
UI menu.c32
TIMEOUT 60
PROMPT 0

DEFAULT live

LABEL live
  KERNEL /live/vmlinuz
  INITRD /live/initrd.img
  APPEND boot=live console=tty1 console=ttyS0,115200

LABEL safe
  KERNEL /live/vmlinuz
  INITRD /live/initrd.img
  APPEND boot=live nomodeset console=tty1 console=ttyS0,115200

LABEL toram
  KERNEL /live/vmlinuz
  INITRD /live/initrd.img
  APPEND boot=live toram console=tty1 console=ttyS0,115200
EOF
  log "isolinux tree added for Rufus ISO-mode"
else
  log "isolinux.bin not found - Rufus ISO-mode (BIOS) files skipped"
fi

log "grub-mkrescue -> hybrid ISO"
grub-mkrescue -o "$OUT/braveos-$VERSION-amd64.iso" "$ISO_TREE" \
  -- -volid BRAVEOS$VERSION -application_id "BraveOS $VERSION Live" >/dev/null 2>&1 \
  || grub-mkrescue -o "$OUT/braveos-$VERSION-amd64.iso" "$ISO_TREE" \
       -- -volid BRAVEOS$VERSION -application_id "BraveOS $VERSION Live"

# -----------------------------------------------------------------------------
# 10. Output
# -----------------------------------------------------------------------------
ISO="$OUT/braveos-$VERSION-amd64.iso"
( cd "$OUT" && sha256sum "braveos-$VERSION-amd64.iso" > SHA256SUMS )
log "ISO: $ISO ($(du -h "$ISO" | cut -f1))"
cat "$OUT/SHA256SUMS"
log "build complete"
