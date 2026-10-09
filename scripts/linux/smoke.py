#!/usr/bin/env python3
import io
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

import gi
from PIL import Image, ImageChops

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi, GLib

binary, out = Path(sys.argv[1]), Path(sys.argv[2])
scale = int(os.environ["GLASSBOARD_X11_DPI"]) / 96
accessibility_scale = int(os.environ.get("GDK_SCALE", "1"))
checks = []
processes = []
handles = []
check_names = (
    "startup refusals", "native welcome and tutorial dismissal", "native drawing and pointer interception",
    "undo", "clear", "toolbar stacking and click-through",
    "capture excludes overlay and retains clipboard ownership", "annotated second copy replaces clipboard",
    "tray Settings and autostart registration",
)


def command(*args, check=True, timeout=10):
    return subprocess.run([str(arg) for arg in args], check=check, capture_output=True, timeout=timeout)


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def wait_for(predicate, message, timeout=15):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(0.1)
    raise AssertionError(message)


def check(name, action):
    try:
        details = action()
    except Exception as error:
        checks.append({"name": name, "status": "fail", "details": str(error)})
        raise
    checks.append({"name": name, "status": "pass", "details": details})
    print(f"PASS {name}", flush=True)
    return details


def start(args, name, env=None):
    handle = (out / f"{name}.log").open("wb")
    handles.append(handle)
    process = subprocess.Popen([str(arg) for arg in args], stdout=handle, stderr=handle, env=env)
    processes.append(process)
    return process


def window(title):
    result = command("xdotool", "search", "--onlyvisible", "--name", f"^{re.escape(title)}$", check=False)
    require(result.returncode in (0, 1), result.stderr.decode())
    matches = result.stdout.splitlines()
    return int(matches[-1]) if matches else None


def geometry(identifier):
    values = dict(line.split("=", 1) for line in command("xdotool", "getwindowgeometry", "--shell", identifier).stdout.decode().splitlines())
    return tuple(int(values[key]) for key in ("X", "Y", "WIDTH", "HEIGHT"))


def screenshot(name):
    path = out / f"{name}.png"
    command("scrot", "--overwrite", path)
    with Image.open(path) as image:
        return image.convert("RGB")


def key(value):
    command("xdotool", "key", "--clearmodifiers", value)
    time.sleep(0.2)


def click(x, y, button=1):
    command("xdotool", "mousemove", round(x), round(y), "click", str(button))


def drag(x1, y1, x2, y2):
    command("xdotool", "mousemove", round(x1), round(y1), "mousedown", "1")
    try:
        for step in range(1, 13):
            command("xdotool", "mousemove", round(x1 + (x2 - x1) * step / 12), round(y1 + (y2 - y1) * step / 12))
            time.sleep(0.025)
    finally:
        command("xdotool", "mouseup", "1")
    time.sleep(0.2)


def accessible(predicate):
    pending = [Atspi.get_desktop(0)]
    while pending:
        node = pending.pop()
        if node is None:
            continue
        try:
            if node.get_role_name() in ("frame", "window", "dialog") and not node.get_state_set().contains(Atspi.StateType.SHOWING):
                continue
            if predicate(node):
                return node
            pending.extend(node.get_child_at_index(index) for index in range(node.get_child_count()))
        except GLib.Error:
            continue
    return None


def control(name, prefix=False):
    def matches(node):
        label = node.get_name()
        named = label.startswith(name) if prefix else label == name
        return named and node.get_state_set().contains(Atspi.StateType.SHOWING) and node.get_role_name() in ("push button", "toggle button", "check box", "switch", "menu item")
    return wait_for(lambda: accessible(matches), f"Accessible control {name!r} did not appear")


def click_control(name, prefix=False):
    node = control(name, prefix)
    bounds = node.get_component_iface().get_extents(Atspi.CoordType.SCREEN)
    require(bounds.width > 0 and bounds.height > 0, f"{name!r} has no screen bounds")
    print(f"CLICK {name!r} at {bounds.x},{bounds.y} {bounds.width}x{bounds.height}", flush=True)
    click((bounds.x + bounds.width / 2) * accessibility_scale, (bounds.y + bounds.height / 2) * accessibility_scale)
    time.sleep(0.2)


def changed_pixels(expected, actual):
    require(expected.size == actual.size, f"Image dimensions differ, expected {expected.size}, got {actual.size}")
    difference = ImageChops.difference(expected, actual)
    return sum(max(pixel) > 20 for pixel in difference.getdata())


def accessibility_snapshot():
    rows = []
    pending = [(Atspi.get_desktop(0), 0)]
    while pending:
        node, depth = pending.pop()
        if node is None:
            continue
        try:
            bounds = node.get_component_iface().get_extents(Atspi.CoordType.SCREEN) if node.get_role_name() == "push button" else None
            position = f" at {bounds.x},{bounds.y} {bounds.width}x{bounds.height}" if bounds else ""
            rows.append(f"{'  ' * depth}{node.get_role_name()} {node.get_name()!r} showing={node.get_state_set().contains(Atspi.StateType.SHOWING)}{position}")
            pending.extend((node.get_child_at_index(index), depth + 1) for index in range(node.get_child_count()))
        except GLib.Error:
            continue
    return "\n".join(rows) + "\n"


def refusals():
    for name, session, display, wayland, expected in (
        ("no-display", "x11", None, None, "DISPLAY"),
        ("wayland-no-display", "wayland", None, "wayland-test", "Wayland"),
        ("xwayland", "wayland", os.environ["DISPLAY"], "wayland-test", "Wayland"),
    ):
        env = os.environ.copy()
        env["XDG_SESSION_TYPE"] = session
        for variable, value in (("DISPLAY", display), ("WAYLAND_DISPLAY", wayland)):
            if value is None:
                env.pop(variable, None)
            else:
                env[variable] = value
        process = start([binary], name, env)
        if display:
            dialog = wait_for(lambda: window("Glassboard could not start"), "Wayland refusal dialog did not appear")
            require(not window("Glassboard annotations"), "Refused session created an overlay")
            screenshot(name)
            command("xdotool", "windowactivate", "--sync", dialog)
            key("Return")
        code = process.wait(timeout=10)
        message = (out / f"{name}.log").read_text()
        require(code == 1 and expected in message, f"{name} returned {code} without the expected explanation")
    return "No display, Wayland without DISPLAY, and XWayland each exited 1 with an explanation."


def fixture_clicks():
    return len(json.loads((out / "fixture.json").read_text())["clicks"])


def toggle(visible):
    key("ctrl+shift+a")
    wait_for(lambda: bool(window("Glassboard annotations")) == visible, "Global drawing shortcut did not change visibility")


def prepare():
    start(["python3", "scripts/linux/fixture.py", out / "fixture.json"], "fixture")
    identifier = wait_for(lambda: window("Glassboard Linux fixture"), "GTK fixture did not appear")
    command("wmctrl", "-i", "-r", identifier, "-e", f"0,{round(100*scale)},{round(160*scale)},{round(850*scale)},{round(450*scale)}")
    wait_for(lambda: (out / "fixture.json").exists(), "Fixture did not paint")
    command("xdotool", "windowactivate", "--sync", identifier)
    start([binary], "glassboard")
    wait_for(lambda: window("Welcome to Glassboard"), "Native welcome window did not appear")
    screenshot("welcome")
    click_control("Dismiss tutorial")
    wait_for(lambda: not window("Welcome to Glassboard"), "Tutorial did not close")
    screenshot("fixture-ready")
    return identifier


def drawing():
    toggle(True)
    overlay = wait_for(lambda: window("Glassboard annotations"), "Drawing overlay did not remain visible")
    command("xdotool", "windowactivate", "--sync", overlay)
    key("ctrl+2")
    key("7")
    baseline = screenshot("drawing-empty").crop(mark_box)
    with Image.open(out / "fixture-ready.png") as reference:
        require(changed_pixels(reference.convert("RGB").crop(mark_box), baseline) < 20, "Empty overlays hid or changed the independent application")
    before = fixture_clicks()
    drag(mark_box[0] + 20 * scale, mark_box[1] + 30 * scale, mark_box[2] - 20 * scale, mark_box[3] - 30 * scale)
    wait_for(lambda: changed_pixels(baseline, screenshot("drawing-mark").crop(mark_box)) > 40,
             "Native pointer drawing did not render")
    painted = screenshot("drawing-mark").crop(mark_box)
    delta = changed_pixels(baseline, painted)
    require(delta > 40, f"Native pointer drawing changed only {delta} pixels")
    require(fixture_clicks() == before, "Drawing leaked pointer input to the independent application")
    return f"Real pointer input changed {delta} pixels without reaching the independent application."


def undo(baseline):
    key("ctrl+z")
    wait_for(lambda: changed_pixels(baseline, screenshot("drawing-undo").crop(mark_box)) < 20,
             "Undo did not restore the fixture region")
    delta = changed_pixels(baseline, screenshot("drawing-undo").crop(mark_box))
    require(delta < 20, f"Undo left {delta} drawing pixels")
    return f"Undo restored the fixture region with {delta} changed pixels."


def clear(baseline):
    drag(mark_box[0] + 20 * scale, mark_box[1] + 30 * scale, mark_box[2] - 20 * scale, mark_box[3] - 30 * scale)
    wait_for(lambda: changed_pixels(baseline, screenshot("drawing-before-clear").crop(mark_box)) > 40,
             "Second mark did not render")
    require(changed_pixels(baseline, screenshot("drawing-before-clear").crop(mark_box)) > 40, "Second mark did not render")
    toolbar = wait_for(lambda: window("Glassboard"), "Toolbar did not appear")
    x, y, width, height = geometry(toolbar)
    print(f"TOOLBAR at {x},{y} {width}x{height}", flush=True)
    command("xdotool", "mousemove", x + width // 2, y + height - round(20 * scale))
    time.sleep(0.5)
    screenshot("toolbar-revealed")
    click_control("Clear this display")
    command("xdotool", "mousemove", 20, 100)
    time.sleep(0.5)
    wait_for(lambda: changed_pixels(baseline, screenshot("drawing-clear").crop(mark_box)) < 20,
             "Clear did not restore the fixture region")
    delta = changed_pixels(baseline, screenshot("drawing-clear").crop(mark_box))
    require(delta < 20, f"Clear left {delta} drawing pixels")
    return "Clear removed the second native pointer mark."


def stacking_and_clickthrough():
    original_bounds = geometry(wait_for(lambda: window("Glassboard"), "Toolbar did not remain visible"))
    for _ in range(3):
        overlay = wait_for(lambda: window("Glassboard annotations"), "Drawing overlay did not remain visible")
        command("xdotool", "windowactivate", "--sync", overlay)
        time.sleep(0.2)
        stack = [int(value, 16) for value in re.findall(r"0x[0-9a-f]+", command("xprop", "-root", "_NET_CLIENT_LIST_STACKING").stdout.decode())]
        toolbar = wait_for(lambda: window("Glassboard"), "Toolbar did not remain visible")
        require(overlay in stack and toolbar in stack and stack.index(toolbar) > stack.index(overlay), "Focused overlay covered the toolbar")
        require(geometry(toolbar) == original_bounds, "Restoring toolbar stacking changed its native bounds")
        active = command("xdotool", "getactivewindow").stdout
        require(int(active) == overlay, "Raising the toolbar stole keyboard focus")
        toggle(False)
        command("xdotool", "windowactivate", "--sync", fixture_id)
        before = fixture_clicks()
        click(mark_box[0] + 30 * scale, mark_box[1] + 30 * scale)
        wait_for(lambda: fixture_clicks() == before + 1, "Hidden overlays blocked a click to the independent application")
        toggle(True)
    screenshot("toolbar-after-focus")
    return "Three focus and hide/show cycles preserved toolbar stacking, focus, and click-through."


def capture(box, annotate, name, expected):
    key("super+ctrl+shift+s")
    editor = wait_for(lambda: window("Glassboard Capture"), "Global screenshot shortcut did not open the native editor")
    command("xdotool", "windowactivate", "--sync", editor)
    drag(*box)
    control("Copy & close", prefix=True)
    if annotate:
        key("ctrl+2")
        key("7")
        drag(box[0] + 30 * scale, box[1] + 30 * scale, box[2] - 30 * scale, box[3] - 30 * scale)
    screenshot(f"{name}-editor")
    key("ctrl+c")
    wait_for(lambda: not window("Glassboard Capture"), "Copy did not close the capture editor")
    png = command("xclip", "-selection", "clipboard", "-target", "image/png", "-out", timeout=10).stdout
    (out / f"{name}-clipboard.png").write_bytes(png)
    with Image.open(io.BytesIO(png)) as image:
        require(image.format == "PNG", "Independent clipboard reader did not receive a PNG")
        actual = image.convert("RGB")
    delta = changed_pixels(expected, actual)
    require(delta > 40 if annotate else delta < 20, f"Unexpected copied pixels, {delta} differ from the independent fixture")
    return f"Independent xclip read a {actual.width} by {actual.height} PNG after editor closure. {delta} pixels differ from the fixture."


def open_settings():
    def tray_icon():
        candidates = []
        def collect(node):
            if node.get_role_name() != "push button" or not node.get_state_set().contains(Atspi.StateType.SHOWING):
                return False
            parent = node.get_parent()
            while parent and parent.get_role_name() != "application":
                parent = parent.get_parent()
            if not parent or parent.get_name() != "xfce4-panel":
                return False
            bounds = node.get_component_iface().get_extents(Atspi.CoordType.SCREEN)
            if bounds.x * accessibility_scale > 640 * scale and 0 <= bounds.y * accessibility_scale < 80 * scale:
                candidates.append(node)
            return False
        accessible(collect)
        return candidates[0] if len(candidates) == 1 else None
    icon = wait_for(tray_icon, "The isolated XFCE tray did not expose one icon button")
    bounds = icon.get_component_iface().get_extents(Atspi.CoordType.SCREEN)
    print(f"TRAY at {bounds.x},{bounds.y} {bounds.width}x{bounds.height}", flush=True)
    click((bounds.x + bounds.width / 2) * accessibility_scale, (bounds.y + bounds.height / 2) * accessibility_scale, 3)
    time.sleep(0.2)
    screenshot("tray-menu")
    key("Home")
    key("Return")
    wait_for(lambda: window("Glassboard Settings"), "Tray Settings did not open")


def autostart():
    open_settings()
    registration = Path.home() / ".config" / "autostart"
    if control("Open at login", prefix=True).get_state_set().contains(Atspi.StateType.CHECKED):
        click_control("Open at login", prefix=True)
        wait_for(lambda: not list(registration.glob("*.desktop")), "Resetting the prior test registration failed")
    require(not list(registration.glob("*.desktop")), "Fresh test home already has an autostart entry")
    click_control("Open at login", prefix=True)
    entries = wait_for(lambda: list(registration.glob("*.desktop")), "Enabling autostart did not create a desktop registration")
    require(len(entries) == 1, "Enabling autostart created multiple registrations")
    entry = entries[0].read_text()
    require("Type=Application" in entry and "glassboard" in entry.lower() and "Exec=" in entry, "Autostart registration is not an application launch entry")
    (out / "autostart.desktop").write_text(entry)
    click_control("Close settings")
    open_settings()
    require(control("Open at login", prefix=True).get_state_set().contains(Atspi.StateType.CHECKED), "Reopened Settings did not read enabled registration")
    screenshot("autostart-enabled")
    click_control("Open at login", prefix=True)
    wait_for(lambda: not list(registration.glob("*.desktop")), "Disabling autostart left its registration")
    click_control("Close settings")
    open_settings()
    require(not control("Open at login", prefix=True).get_state_set().contains(Atspi.StateType.CHECKED), "Reopened Settings did not read disabled registration")
    screenshot("autostart-disabled")
    click_control("Close settings")
    return "Tray menu opened Settings. Autostart enabled, reread its desktop registration, disabled, and reread the absence."


failed = False
try:
    check("startup refusals", refusals)
    fixture_id = check("native welcome and tutorial dismissal", prepare)
    fx, fy, fw, fh = geometry(fixture_id)
    mark_box = tuple(round(value) for value in (fx + 40 * scale, fy + 40 * scale, fx + 430 * scale, fy + 160 * scale))
    check("native drawing and pointer interception", drawing)
    with Image.open(out / "drawing-empty.png") as image:
        baseline = image.convert("RGB").crop(mark_box)
    check("undo", lambda: undo(baseline))
    check("toolbar stacking and click-through", stacking_and_clickthrough)
    check("clear", lambda: clear(baseline))
    toggle(False)
    reference = screenshot("independent-capture-reference")
    first_box = tuple(round(value) for value in (fx + 40 * scale, fy + 40 * scale, fx + 430 * scale, fy + 160 * scale))
    second_box = tuple(round(value) for value in (fx + 50 * scale, fy + 230 * scale, fx + 600 * scale, fy + 380 * scale))
    toggle(True)
    drag(first_box[0] + 20 * scale, first_box[1] + 30 * scale, first_box[2] - 20 * scale, first_box[3] - 30 * scale)
    check("capture excludes overlay and retains clipboard ownership", lambda: capture(first_box, False, "first-copy", reference.crop(first_box)))
    check("annotated second copy replaces clipboard", lambda: capture(second_box, True, "second-copy", reference.crop(second_box)))
    check("tray Settings and autostart registration", autostart)
except Exception as error:
    failed = True
    print(f"FAIL {error}", file=sys.stderr, flush=True)
    screenshot("failure")
    (out / "windows.txt").write_bytes(command("wmctrl", "-lG", check=False).stdout)
    (out / "stacking.txt").write_bytes(command("xprop", "-root", "_NET_CLIENT_LIST_STACKING", check=False).stdout)
    (out / "accessibility.txt").write_text(accessibility_snapshot())
finally:
    for process in reversed(processes):
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
    for handle in handles:
        handle.close()
    observed = {result["name"] for result in checks}
    checks.extend({"name": name, "status": "unverified", "details": "An earlier check failed."} for name in check_names if name not in observed)
    (out / "results.json").write_text(json.dumps({
        "binary": str(binary),
        "dpi": int(os.environ["GLASSBOARD_X11_DPI"]),
        "gdkScale": int(os.environ.get("GDK_SCALE", "1")),
        "status": "fail" if failed else "pass",
        "checks": checks,
        "unverified": ["Physical GPUs and monitors", "Multiple monitors and negative origins", "Other X11 window managers and tray hosts", "Native Wayland GUI refusal", "Login session autostart execution", "Full-monitor copy", "Signed AppImage update installation"] + ([] if binary.suffix == ".AppImage" else ["Packaged AppImage launch"]),
    }, indent=2) + "\n")
sys.exit(1 if failed else 0)
