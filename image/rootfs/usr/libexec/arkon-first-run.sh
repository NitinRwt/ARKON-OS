#!/usr/bin/env bash
# Runs once on first KDE login to apply ARKON theme settings.
set -euo pipefail

MARKER="${HOME}/.config/arkon-first-run"
[[ -f "${MARKER}" ]] && exit 0

# Apply color scheme
plasma-apply-colorscheme ArkonDark 2>/dev/null || true

# Apply Breeze Dark look-and-feel as base (closest to macOS clean dark)
plasma-apply-lookandfeel --apply org.kde.breezedark.desktop 2>/dev/null || true

# Apply wallpaper
plasma-apply-wallpaperimage /usr/share/wallpapers/arkon/contents/images/3840x2160.svg 2>/dev/null || true

# Set icon theme to Papirus-Dark
kwriteconfig6 --file kdeglobals --group Icons --key Theme Papirus-Dark 2>/dev/null || true

# Enable blur effect
kwriteconfig6 --file kwinrc --group Plugins --key blurEnabled true 2>/dev/null || true

# Reload KWin and Plasma config
qdbus6 org.kde.KWin /KWin reconfigure 2>/dev/null || true
qdbus6 org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.refreshCurrentShell 2>/dev/null || true

# Mark done
touch "${MARKER}"
