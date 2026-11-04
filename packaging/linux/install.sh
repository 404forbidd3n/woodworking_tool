#!/usr/bin/env bash
# Install Woodworking for the current user. No root needed.
#
#   ./install.sh              install or update
#   ./install.sh --uninstall  remove the app (your projects in ~/Woodworking stay)
#
# What goes where:
#   ~/.local/share/woodworking-app/woodworking     the program
#   ~/.local/bin/woodworking                       a link, so `woodworking` runs it
#   ~/.local/share/applications/woodworking.desktop  the app-menu entry
#   ~/Woodworking/                                 projects and customs.json
#
# Any *.json shipped next to this script is copied into ~/Woodworking, but
# only when a file with that name is not already there, so updating never
# overwrites work.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
app_dir="$HOME/.local/share/woodworking-app"
bin_link="$HOME/.local/bin/woodworking"
desktop="$HOME/.local/share/applications/woodworking.desktop"
projects="$HOME/Woodworking"

if [[ "${1:-}" == "--uninstall" ]]; then
    rm -rf "$app_dir" "$bin_link" "$desktop"
    echo "Removed Woodworking. Your projects in $projects were kept."
    exit 0
fi

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
    echo "This package is for 64-bit Intel/AMD Linux (x86_64)."
    echo "This computer is $(uname -s) $(uname -m)."
    exit 1
fi

if [[ ! -f "$here/woodworking" ]]; then
    echo "The 'woodworking' program is missing next to install.sh."
    exit 1
fi

mkdir -p "$app_dir" "$(dirname "$bin_link")" "$(dirname "$desktop")" "$projects"
install -m 755 "$here/woodworking" "$app_dir/woodworking"
ln -sf "$app_dir/woodworking" "$bin_link"

shopt -s nullglob
for file in "$here"/*.json; do
    name="$(basename "$file")"
    if [[ ! -e "$projects/$name" ]]; then
        cp "$file" "$projects/$name"
        echo "Added $name to $projects"
    fi
done

cat > "$desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Woodworking
Comment=3D layout tool for kitchens and cabinets
Exec=$app_dir/woodworking
Path=$projects
Icon=applications-engineering
Terminal=false
Categories=Graphics;Engineering;
EOF

# Warn about missing system libraries instead of failing on first launch.
missing="$(ldd "$app_dir/woodworking" 2>/dev/null | awk '/not found/ {print $1}' || true)"
if [[ -n "$missing" ]]; then
    echo
    echo "Some system libraries are missing:"
    echo "$missing" | sed 's/^/  /'
    echo "On Ubuntu/Debian:  sudo apt install libasound2 libudev1 libxkbcommon0 libgtk-3-0 mesa-vulkan-drivers"
    echo "On Fedora:         sudo dnf install alsa-lib systemd-libs libxkbcommon gtk3 mesa-vulkan-drivers"
    echo "On Arch:           sudo pacman -S alsa-lib systemd-libs libxkbcommon gtk3 vulkan-icd-loader"
fi

echo
echo "Installed. Start it from the app menu (\"Woodworking\") or run: woodworking"
echo "Projects are saved in $projects"
case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *) echo "(~/.local/bin is not on your PATH, so use the app menu or $app_dir/woodworking)" ;;
esac
