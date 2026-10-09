#!/usr/bin/env bash
set -euo pipefail

if [ "${1:-}" != desktop ]; then
    npm ci
    npm run check
    npm test
    npm run build
    cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml
    cargo clippy --locked --all-targets --manifest-path apps/desktop/src-tauri/Cargo.toml -- -D warnings
    npm --workspace apps/desktop run tauri -- build --debug --no-bundle
    binary="$CARGO_TARGET_DIR/debug/glassboard"
    test -x "$binary"
    for dpi in ${GLASSBOARD_LINUX_DPI:-96 192}; do
        GLASSBOARD_X11_DPI="$dpi" dbus-run-session -- bash scripts/linux/container.sh desktop "$binary"
    done
    exit
fi

dpi=${GLASSBOARD_X11_DPI:?}
case "$dpi" in ''|*[!0-9]*) echo 'DPI must be a positive integer.' >&2; exit 1;; esac
test "$dpi" -ge 96
width=$((1280 * dpi / 96))
height=$((800 * dpi / 96))
task_home=$(mktemp -d "/tmp/glassboard-home-$dpi.XXXXXX")
export XDG_CONFIG_HOME="$task_home/.config" XDG_DATA_HOME="$task_home/.local/share"
export XDG_CACHE_HOME="$task_home/.cache" XDG_RUNTIME_DIR="$task_home/runtime"
export XDG_SESSION_TYPE=x11 GDK_BACKEND=x11
export GDK_SCALE=${GLASSBOARD_LINUX_GDK_SCALE:-$((dpi / 96))}
export GDK_DPI_SCALE=1
export NO_AT_BRIDGE=0 GTK_MODULES=atk-bridge
unset WAYLAND_DISPLAY
mkdir -p "$XDG_CONFIG_HOME/xfce4/xfconf/xfce-perchannel-xml" "$XDG_RUNTIME_DIR" "/evidence/dpi-$dpi"
chmod 700 "$XDG_RUNTIME_DIR"
out="/evidence/dpi-$dpi"
cat > "$XDG_CONFIG_HOME/xfce4/xfconf/xfce-perchannel-xml/xfce4-panel.xml" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<channel name="xfce4-panel" version="1.0">
  <property name="configver" type="int" value="2"/>
  <property name="panels" type="array">
    <value type="int" value="1"/>
    <property name="panel-1" type="empty">
      <property name="position" type="string" value="p=0;x=$((width - 40));y=20"/>
      <property name="position-locked" type="bool" value="true"/>
      <property name="size" type="uint" value="32"/>
      <property name="length" type="uint" value="5"/>
      <property name="plugin-ids" type="array"><value type="int" value="1"/></property>
    </property>
  </property>
  <property name="plugins" type="empty"><property name="plugin-1" type="string" value="systray"/></property>
</channel>
EOF

pids=()
trap 'for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done; for pid in "${pids[@]}"; do wait "$pid" 2>/dev/null || true; done' EXIT
Xvfb -displayfd 3 -screen 0 "${width}x${height}x24" -dpi "$dpi" -nolisten tcp 3> "$task_home/display" > "$out/xvfb.log" 2>&1 &
pids+=($!)
for attempt in {1..100}; do
    if [ -s "$task_home/display" ]; then break; fi
    sleep 0.1
done
test -s "$task_home/display"
export DISPLAY=":$(cat "$task_home/display")"
for attempt in {1..100}; do
    if xdpyinfo >/dev/null 2>&1; then break; fi
    sleep 0.1
done
xdpyinfo > "$out/display.txt"
printf 'Xft.dpi: %s\n' "$dpi" | xrdb -merge
gsettings set org.gnome.desktop.interface toolkit-accessibility true
openbox > "$out/openbox.log" 2>&1 &
pids+=($!)
for attempt in {1..100}; do
    if wmctrl -m >/dev/null 2>&1; then break; fi
    sleep 0.1
done
wmctrl -m > "$out/window-manager.txt"
xcompmgr -n > "$out/compositor.log" 2>&1 &
pids+=($!)
xfce4-panel --disable-wm-check > "$out/panel.log" 2>&1 &
pids+=($!)
python3 scripts/linux/smoke.py "$2" "$out"
