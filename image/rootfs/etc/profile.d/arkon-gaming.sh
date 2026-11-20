#!/usr/bin/env bash
# ARKON gaming environment — sets Steam + Proton runtime variables.

# Allow Steam to use XWayland (Steam's native Wayland support is still buggy
# for many titles; XWayland gives better compatibility).
export STEAM_FORCE_DESKTOPUI_SCALING=1

# Tell SDL to prefer Wayland but fall back to X11 for titles that need it.
# Games run via Proton always use XWayland anyway.
export SDL_VIDEODRIVER=wayland,x11

# Enable GameMode globally — games that call libgamemode will auto-activate it.
export LD_PRELOAD="${LD_PRELOAD:+$LD_PRELOAD:}/usr/\$LIB/libgamemode.so.0"

# MangoHud — press F12 in-game to toggle overlay.
export MANGOHUD=0

# Proton: enable DXVK async shader compilation (reduces stuttering).
export DXVK_ASYNC=1

# CS2 / Source 2: prefer Vulkan over OpenGL.
export VKD3D_FEATURE_LEVEL=12_1
