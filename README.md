# BraveOS — the Brave Browser Distribution

**BraveOS** is a minimal, custom-built Linux distribution built around a single
idea: *the browser is the computer.* It ships **only** the Brave browser, a
file manager and a custom settings app on a clean Debian base, and gets out of
the way.

> ⚠️ Unofficial community project. Brave® is a trademark of Brave Software
> Inc.; this project simply packages the official `brave-browser` build from
> Brave's own repository. Not affiliated with Brave Software.

## What's inside (and nothing else)

| Component | Choice | Idle RAM |
|---|---|---|
| Base | Debian 12 (bookworm) `minbase` debootstrap — no distro layer | — |
| Kernel | Linux 6.1 LTS (Debian build, full driver suite) | — |
| Session | Xorg + Openbox + picom compositor + tint2 panel | ~90–130 MB |
| Browser | Brave (official release repo) | on demand |
| Files | PCManFM (also draws the desktop/wallpaper) | on demand |
| Settings | **custom GTK app** (`bravos-settings`, ~600 lines) | on demand |
| Installer | **custom GTK wizard** (`bravos-installer`) | on demand |
| Sound | PulseAudio + ALSA | ~12 MB |
| Network | NetworkManager + nm-applet + wpa_supplicant | ~15 MB |

The full boot-to-login idle footprint (kernel + session, no apps open) is
measured in CI on every build — see the `idle-ram` line in the smoke-test log
and the `ram-report.txt` artifact.

## Hardware support (target: 2008–2022)

- **Intel CPUs/GPUs:** i915 from GMA X4500 / Atom (2008) through Alder Lake
  (2022); video decode via `i965-va-driver` + `intel-media-va-driver`.
- **AMD CPUs/GPUs:** `amdgpu` (GCN 1.0, HD 7000 series 2012, → RDNA2 2022)
  and `radeon` (pre-GCN, HD 2000+ 2007); both firmwares included.
- **NVIDIA GPUs:** `nouveau` (G80 / GeForce 8000 series, 2008 → Turing 2018).
  Proprietary NVIDIA driver is intentionally *not* bundled (it is the #1 cause
  of black screens in custom distros); `nomodeset` menu entry exists as an
  emergency fallback.
- **Network:** Intel e1000/e1000e/iwlwifi (3945ABG→AX210), Realtek r8169/rtl8xxxu
  /rtw88/rtw89, Atheros ath5k/ath9k/ath10k, Broadcom b43/tg3, plus most USB
  dongles via `firmware-realtek`/`firmware-misc-nonfree`.
- Audio: HDA Intel/Realtek/AMD, SOF (2018+).

## Custom UI

- Dark "Brave" GTK theme with brand-orange (#f04e23) accents, custom Openbox
  window theme, custom wallpapers/GRUB background and icon set (all generated
  by `scripts/gen_assets.py` — sources in `assets/icons/*.svg`).
- Compositing: shadows + **fluid fade animations** (picom), smooth desktop/
  workspace transitions.
- **Custom Settings app** with sidebar navigation: About, Display, Network,
  Sound, Brave, Memory & Storage, Power & Tuning, Live Session.
- **Custom start page** (`/usr/share/bravos/start/start.html`) with clock,
  search and quick tiles.
- Top panel with Brave/Files/Settings/Installer launchers, taskbar, systray
  and clock.

## Resource efficiency

- `vm.swappiness=10`, `vfs_cache_pressure=50` — the page cache is left to the
  browser.
- zram (zstd, 50 % of RAM) compressed swap instead of disk swapping.
- journald kept volatile (RAM, capped at 16 MB), no docs/man pages/locales
  beyond English in the image.
- No display manager, no background daemons beyond NetworkManager/PulseAudio.
- Brave launcher wrapper (`/usr/local/bin/brave`) applies hardware-
  acceleration / kiosk / RAM-cache flags from `~/.config/bravos/brave.flags`.

## Live session & installer

The ISO boots live from USB (BIOS **and** UEFI; *disable Secure Boot for
v1*). Boot menu entries:

1. Live session
2. Live session (safe graphics — `nomodeset`)
3. Live session (copy to RAM — `toram`, fastest)
4. **Install BraveOS to disk (guided)** — same as the desktop icon
5. Live session (verbose boot output)

The guided installer (desktop icon “Install BraveOS” or Settings → Live
Session) erases one disk of your choice, creates GPT (512 MiB ESP + 1 MiB
BIOS-boot + root ext4), rsyncs the live root, installs GRUB for BIOS and UEFI
(removable fallback path included) and sets your hostname/password. Nothing
else — no telemetry, no network needed.

Live login: user `brave`, password `brave` (passwordless sudo in the live
session only; the installed system asks for the password you set).

## Building it yourself

Everything builds on GitHub Actions (see `.github/workflows/build.yml`):
push → `debootstrap` → apt (incl. official Brave repo) → overlay → zstd
squashfs → `grub-mkrescue` hybrid ISO → QEMU smoke test (KVM) → artifacts.

```bash
# locally (needs root, Debian/Ubuntu host):
sudo bash build.sh
```

## Known limitations (v1.0.0)

- Secure Boot machines need it disabled in firmware setup (unsigned GRUB).
- Rootfs squashfs ≈ 0.8–1 GB ISO — firmware blobs for 14 years of hardware
  are the bulk of it.
- English UI only; no CJK fonts preinstalled (add via Brave or APT post-install).
- GUI button *clicks* are not machine-verified in CI; the smoke test does
  verify boot, login, session processes, idle RAM, self-tests of both custom
  apps and screenshots of the real display.

## Repository layout

```
build.sh               ISO build orchestrator (runs as root, on CI or locally)
config/packages.list   exact package set
config/initial_preferences  Brave start page + sane defaults
grub/grub.cfg          live boot menu
scripts/gen_assets.py  wallpapers, GRUB bg, icon rasterizer
scripts/smoke_test.py  QEMU boot test + screenshots + RAM report
rootfs/                everything overlaid into the image (apps, themes, configs)
```
