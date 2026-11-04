#!/usr/bin/env python3
"""Installs a Glassboard NSIS build and checks it with real input on a Windows desktop.

Usage: smoke.py <setup.exe> <evidence directory>
"""
import ctypes
import json
import os
import subprocess
import sys
import time
import traceback
import winreg
from contextlib import contextmanager
from ctypes import wintypes
from pathlib import Path

from PIL import Image, ImageChops, ImageGrab

setup, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
out.mkdir(parents=True, exist_ok=True)
here = Path(__file__).resolve().parent

IDENTIFIER = "dev.glassboard.desktop"
INSTALL_DIR = Path(os.environ["LOCALAPPDATA"]) / "Glassboard"
BINARY = INSTALL_DIR / "glassboard.exe"
UNINSTALLER = INSTALL_DIR / "uninstall.exe"
CONFIG_DIR = Path(os.environ["APPDATA"]) / IDENTIFIER
LOG_DIR = Path(os.environ["LOCALAPPDATA"]) / IDENTIFIER / "logs"
PREFERENCES = CONFIG_DIR / "preferences.json"
RUN_KEY = r"Software\Microsoft\Windows\CurrentVersion\Run"

OVERLAY = "Glassboard annotations"
TOOLBAR = "Glassboard"
SETTINGS = "Glassboard Settings"
CAPTURE = "Glassboard Capture"
FIXTURE = "Glassboard Windows fixture"
# Fixture coordinates in screen pixels, clear of the toolbar docked at the bottom.
MARK_BOX = (150, 120, 650, 420)

user32 = ctypes.WinDLL("user32", use_last_error=True)
kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
WNDENUMPROC = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
user32.EnumWindows.argtypes = [WNDENUMPROC, wintypes.LPARAM]
user32.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
user32.IsWindowVisible.argtypes = [wintypes.HWND]
user32.GetWindow.argtypes = [wintypes.HWND, wintypes.UINT]
user32.GetWindow.restype = wintypes.HWND
user32.GetForegroundWindow.restype = wintypes.HWND
user32.GetWindowRect.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.RECT)]
user32.GetSystemMetrics.argtypes = [ctypes.c_int]
user32.keybd_event.argtypes = [wintypes.BYTE, wintypes.BYTE, wintypes.DWORD, ctypes.c_size_t]
user32.mouse_event.argtypes = [wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, ctypes.c_size_t]
kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
kernel32.OpenProcess.restype = wintypes.HANDLE
kernel32.QueryFullProcessImageNameW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]
kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
kernel32.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, wintypes.LPVOID, wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
kernel32.CreateFileW.restype = wintypes.HANDLE
user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))

GW_HWNDPREV = 3
KEYUP = 2
MOUSE_MOVE_ABSOLUTE = 0x8001
LEFT_DOWN, LEFT_UP, RIGHT_DOWN, RIGHT_UP = 0x2, 0x4, 0x8, 0x10
VK = {"ctrl": 0x11, "shift": 0x10, "alt": 0x12, "win": 0x5B, "tab": 0x09, "esc": 0x1B, "f3": 0x72, "f4": 0x73, "f5": 0x74}
EXTENDED = {0x5B}

results = []
processes = []
current_check = ""


class Inconclusive(Exception):
    pass


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def wait_for(predicate, message, timeout=20):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(0.1)
    raise AssertionError(message)


def check(name, action):
    global current_check
    current_check = name
    print(f"RUN  {name}", flush=True)
    try:
        details = action() or ""
        status = "pass"
    except AssertionError as error:
        status, details = "fail", str(error)
    except Inconclusive as error:
        status, details = "inconclusive", str(error)
    except Exception:
        status, details = "inconclusive", f"Harness error:\n{traceback.format_exc()}"
    print(f"{status.upper():<5}{name}: {details}", flush=True)
    results.append({"name": name, "status": status, "details": details})
    write_results()


def slug(name):
    return "".join(c if c.isalnum() else "-" for c in name.lower())


def write_results():
    (out / "results.json").write_text(json.dumps(results, indent=2) + "\n")


def screenshot(name):
    image = ImageGrab.grab().convert("RGB")
    image.save(out / f"{name}.png")
    return image


def changed_pixels(first, second):
    difference = ImageChops.difference(first, second).convert("L")
    return sum(1 for value in difference.getdata() if value > 16)


def press(*names):
    codes = [VK[name] if name in VK else ord(name.upper()) for name in names]
    for code in codes:
        user32.keybd_event(code, 0, 1 if code in EXTENDED else 0, 0)
        time.sleep(0.02)
    for code in reversed(codes):
        user32.keybd_event(code, 0, (1 if code in EXTENDED else 0) | KEYUP, 0)
        time.sleep(0.02)


def move(x, y):
    width, height = user32.GetSystemMetrics(0), user32.GetSystemMetrics(1)
    user32.mouse_event(MOUSE_MOVE_ABSOLUTE, round((x + 0.5) * 65536 / width), round((y + 0.5) * 65536 / height), 0, 0)


def click(x, y, down=LEFT_DOWN, up=LEFT_UP):
    move(x, y)
    time.sleep(0.15)
    user32.mouse_event(down, 0, 0, 0, 0)
    time.sleep(0.05)
    user32.mouse_event(up, 0, 0, 0, 0)
    time.sleep(0.1)


def drag(x1, y1, x2, y2):
    move(x1, y1)
    time.sleep(0.15)
    user32.mouse_event(LEFT_DOWN, 0, 0, 0, 0)
    for step in range(1, 25):
        move(x1 + (x2 - x1) * step / 24, y1 + (y2 - y1) * step / 24)
        time.sleep(0.016)
    user32.mouse_event(LEFT_UP, 0, 0, 0, 0)
    time.sleep(0.2)


def process_image(pid):
    handle = kernel32.OpenProcess(0x1000, False, pid)
    if not handle:
        return ""
    try:
        size = wintypes.DWORD(1024)
        buffer = ctypes.create_unicode_buffer(1024)
        kernel32.QueryFullProcessImageNameW(handle, 0, buffer, ctypes.byref(size))
        return Path(buffer.value).name.lower()
    finally:
        kernel32.CloseHandle(handle)


def top_level_windows():
    """Every visible top-level window, top of the Z order first."""
    found = []

    def visit(hwnd, _):
        if user32.IsWindowVisible(hwnd):
            title = ctypes.create_unicode_buffer(256)
            user32.GetWindowTextW(hwnd, title, 256)
            pid = wintypes.DWORD()
            user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
            found.append({"hwnd": hwnd, "title": title.value, "pid": pid.value, "image": process_image(pid.value)})
        return True

    user32.EnumWindows(WNDENUMPROC(visit), 0)
    return found


def visible(title):
    return next((window["hwnd"] for window in top_level_windows() if window["title"] == title and window["image"] == "glassboard.exe"), None)


def rectangle(hwnd):
    rect = wintypes.RECT()
    require(user32.GetWindowRect(hwnd, ctypes.byref(rect)), "Could not read window bounds")
    return rect.left, rect.top, rect.right, rect.bottom


def above(top, bottom):
    previous = user32.GetWindow(bottom, GW_HWNDPREV)
    while previous:
        if previous == top:
            return True
        previous = user32.GetWindow(previous, GW_HWNDPREV)
    return False


def webview_windows():
    return {window["hwnd"]: window["title"] for window in top_level_windows() if window["image"] == "msedgewebview2.exe"}


def seed_preferences(**overrides):
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    PREFERENCES.write_text(json.dumps({"version": 1, "tutorialCompleted": True, "tool": "pen", "colorMode": "solid", "language": "en", **overrides}))


def stop_glassboard():
    subprocess.run(["taskkill", "/F", "/T", "/IM", "glassboard.exe"], capture_output=True)
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        listing = subprocess.run(["tasklist", "/FI", "IMAGENAME eq glassboard.exe"], capture_output=True, text=True).stdout
        if "glassboard.exe" not in listing.lower():
            return
        time.sleep(0.2)
    raise Inconclusive("Glassboard did not exit")


def hold_preferences():
    """Antivirus and indexers briefly hold new files open, which blocks replacing them."""
    handle = kernel32.CreateFileW(str(PREFERENCES), 0x80000000, 0, None, 3, 0x80, None)
    require(handle != wintypes.HANDLE(-1).value, f"Could not hold preferences.json: {ctypes.get_last_error()}")
    return handle


@contextmanager
def glassboard(held_at_launch=False, **preferences):
    stop_glassboard()
    seed_preferences(**preferences)
    held = hold_preferences() if held_at_launch else None
    process = subprocess.Popen([str(BINARY)])
    if held:
        time.sleep(1)
        kernel32.CloseHandle(held)
    try:
        # The drawing window exists hidden, so wait for its webview process to settle.
        wait_for(lambda: tray_process_ready(process), "Glassboard exited while starting")
        time.sleep(5)
        if not held_at_launch:
            focus_fixture()
        yield process
    except BaseException:
        record_failure(process)
        raise
    finally:
        stop_glassboard()


def record_failure(process):
    """Runs before Glassboard is stopped, so this shows the state the check failed in."""
    screenshot(f"failed-{slug(current_check)}")
    foreground = ctypes.create_unicode_buffer(256)
    user32.GetWindowTextW(user32.GetForegroundWindow(), foreground, 256)
    windows = [(window["title"], window["image"]) for window in top_level_windows() if "glassboard" in window["image"] or window["title"] == FIXTURE]
    with (out / "diagnostics.txt").open("a") as file:
        file.write(f"== {current_check}\nexit code: {process.poll()}\nforeground: {foreground.value!r}\nwindows: {windows}\nlog tail:\n{log_text()[-800:]}\n\n")


def tray_process_ready(process):
    return process.poll() is None


def clicks():
    return int((out / "fixture-clicks.txt").read_text() or 0)


def focus_fixture():
    hwnd = wait_for(lambda: next((window["hwnd"] for window in top_level_windows() if window["title"] == FIXTURE), None), "Fixture is not on screen")
    if user32.GetForegroundWindow() == hwnd:
        return hwnd
    left, top, right, _ = rectangle(hwnd)
    # The title bar takes focus without counting as a click on the fixture canvas.
    click((left + right) // 2, top + 12)
    wait_for(lambda: user32.GetForegroundWindow() == hwnd, "Fixture did not take focus", timeout=5)
    return hwnd


def toggle(shown):
    press("ctrl", "shift", "a")
    wait_for(lambda: bool(visible(OVERLAY)) == shown, f"The global shortcut did not {'show' if shown else 'hide'} the drawing window")
    time.sleep(0.8 if shown else 0.3)


def draw_mark():
    before = ImageGrab.grab().convert("RGB").crop(MARK_BOX)
    drag(MARK_BOX[0] + 30, MARK_BOX[1] + 40, MARK_BOX[2] - 30, MARK_BOX[3] - 40)
    time.sleep(0.5)
    after = ImageGrab.grab().convert("RGB").crop(MARK_BOX)
    delta = changed_pixels(before, after)
    require(delta > 40, f"Drawing changed only {delta} pixels")
    return after


def park_pointer():
    move(MARK_BOX[0] + 10, MARK_BOX[1] + 10)
    time.sleep(0.2)


def powershell_uia(script, **env):
    prelude = (
        "Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes;"
        "$A=[System.Windows.Automation.AutomationElement];"
        "$T=[System.Windows.Automation.TreeScope];"
        "$Any=[System.Windows.Automation.Condition]::TrueCondition;"
        "function Rect($e){$r=$e.Current.BoundingRectangle;Write-Output \"$($r.X),$($r.Y),$($r.Width),$($r.Height)\"};"
    )
    result = subprocess.run(
        ["powershell", "-NoProfile", "-Command", prelude + script],
        capture_output=True, text=True, timeout=60, env={**os.environ, **env},
    )
    return result.stdout.strip()


def uia_center(window_title, name):
    """The screen center of the named control in a window, read from UI Automation."""
    found = powershell_uia(
        "$w=$A::RootElement.FindFirst($T::Children,(New-Object System.Windows.Automation.PropertyCondition($A::NameProperty,$env:GB_WINDOW)));"
        "if($w){foreach($e in $w.FindAll($T::Descendants,$Any)){if($e.Current.Name -like \"$env:GB_NAME*\" -and $e.Current.ControlType.ProgrammaticName -match 'Button|CheckBox'){Rect $e;break}}}",
        GB_WINDOW=window_title, GB_NAME=name,
    )
    if not found:
        return None
    x, y, width, height = (float(part) for part in found.split(","))
    return round(x + width / 2), round(y + height / 2)


def promote_tray_icon():
    """Windows hides new tray icons in an overflow flyout. Pin Glassboard's to the taskbar."""
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, r"Control Panel\NotifyIconSettings") as root:
            for index in range(winreg.QueryInfoKey(root)[0]):
                name = winreg.EnumKey(root, index)
                with winreg.OpenKey(root, name, 0, winreg.KEY_READ | winreg.KEY_SET_VALUE) as entry:
                    try:
                        path, _ = winreg.QueryValueEx(entry, "ExecutablePath")
                    except FileNotFoundError:
                        continue
                    if path.lower().endswith("glassboard.exe"):
                        winreg.SetValueEx(entry, "IsPromoted", 0, winreg.REG_DWORD, 1)
    except FileNotFoundError:
        pass


def tray_icon():
    found = powershell_uia(
        "$taskbar=New-Object System.Windows.Automation.PropertyCondition($A::ClassNameProperty,'Shell_TrayWnd');"
        "$bar=$A::RootElement.FindFirst($T::Children,$taskbar);"
        "foreach($e in $bar.FindAll($T::Descendants,$Any)){if($e.Current.Name -like 'Glassboard*'){Rect $e;break}}"
    )
    if not found:
        return None
    x, y, width, height = (float(part) for part in found.split(","))
    return round(x + width / 2), round(y + height / 2)


def find_tray_icon():
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        promote_tray_icon()
        center = tray_icon()
        if center:
            return center
        time.sleep(1)
    names = powershell_uia(
        "$taskbar=New-Object System.Windows.Automation.PropertyCondition($A::ClassNameProperty,'Shell_TrayWnd');"
        "$bar=$A::RootElement.FindFirst($T::Children,$taskbar);"
        "foreach($e in $bar.FindAll($T::Descendants,$Any)){Write-Output \"$($e.Current.ClassName)|$($e.Current.Name)\"}"
    )
    (out / "taskbar-tree.txt").write_text(names)
    screenshot("tray-not-found")
    raise Inconclusive("The taskbar did not expose a Glassboard icon. See taskbar-tree.txt.")


def install():
    subprocess.run([str(setup), "/S"], check=True, timeout=180)
    wait_for(BINARY.exists, f"The installer did not create {BINARY}", timeout=60)
    require(UNINSTALLER.exists(), f"The installer did not create {UNINSTALLER}")
    return f"Installed {setup.name} to {INSTALL_DIR}."


def start_fixture():
    processes.append(subprocess.Popen([sys.executable, str(here / "fixture.py"), str(out / "fixture-clicks.txt")]))
    wait_for(lambda: any(window["title"] == FIXTURE for window in top_level_windows()), "Fixture did not appear")
    time.sleep(1)


def draws_over_other_apps():
    with glassboard():
        reference = screenshot("reference-hidden")
        before = clicks()
        toggle(True)
        require(visible(TOOLBAR), "The toolbar did not appear with the drawing window")
        draw_mark()
        screenshot("drawing")
        require(clicks() == before, "Drawing leaked pointer input to the app underneath")
        press("ctrl", "z")
        time.sleep(0.5)
        remaining = changed_pixels(reference.crop(MARK_BOX), ImageGrab.grab().convert("RGB").crop(MARK_BOX))
        require(remaining < 20, f"Undo left {remaining} pixels behind")
        toggle(False)
        click(MARK_BOX[0] + 30, MARK_BOX[1] + 30)
        wait_for(lambda: clicks() == before + 1, "The hidden drawing window blocked a click to the app underneath", timeout=5)
    return "Drew with real pointer input, undid it, and clicks reached the app underneath once hidden."


def toolbar_stays_on_top():
    with glassboard():
        toggle(True)
        bounds = rectangle(visible(TOOLBAR))
        for _ in range(3):
            click(MARK_BOX[0] + 40, MARK_BOX[1] + 40)
            time.sleep(0.3)
            overlay, toolbar = visible(OVERLAY), visible(TOOLBAR)
            require(above(toolbar, overlay), "The drawing window covered the toolbar after taking focus")
            require(rectangle(toolbar) == bounds, "Raising the toolbar moved it")
            require(user32.GetForegroundWindow() == overlay, "Raising the toolbar took focus from the drawing window")
    return "The toolbar stayed above the focused drawing window for three clicks."


def focus_returns_to_previous_app():
    with glassboard():
        fixture = focus_fixture()
        toggle(True)
        draw_mark()
        toggle(False)
        time.sleep(0.5)
        foreground = user32.GetForegroundWindow()
        title = ctypes.create_unicode_buffer(256)
        user32.GetWindowTextW(foreground, title, 256)
        require(foreground == fixture, f"Focus went to {title.value!r} after drawing, not back to the previous app")
    return "Focus returned to the previous app when drawing ended."


def browser_key_is_inert(*keys):
    def run():
        with glassboard():
            toggle(True)
            draw_mark()
            park_pointer()
            before = screenshot(f"before-{'-'.join(keys)}")
            press(*keys)
            time.sleep(3)
            after = screenshot(f"after-{'-'.join(keys)}")
            left, top, right, bottom = rectangle(visible(TOOLBAR)) if visible(TOOLBAR) else (0, 0, 0, 0)
            # The toolbar animates independently, so only pixels outside it count.
            for image in (before, after):
                image.paste((0, 0, 0), (left, top, right, bottom))
            delta = changed_pixels(before, after)
            windows = list(webview_windows().values())
            require(delta < 40, f"{'+'.join(keys)} changed {delta} pixels on screen. Browser windows: {windows}")
        return f"{'+'.join(keys)} left the drawing untouched."
    return run


def right_click_stays_inert(surface_click, name):
    before_windows = webview_windows()
    before = screenshot(f"before-menu-{name}")
    surface_click()
    time.sleep(1.2)
    after = screenshot(f"after-menu-{name}")
    opened = {hwnd: title for hwnd, title in webview_windows().items() if hwnd not in before_windows}
    delta = changed_pixels(before, after)
    require(not opened, f"Right-clicking the {name} opened a browser window. {delta} pixels changed. See after-menu-{name}.png.")


def no_browser_menu_on_toolbar():
    def right_click_toolbar():
        left, top, right, bottom = rectangle(visible(TOOLBAR))
        move((left + right) // 2, bottom - 30)
        time.sleep(0.6)
        click((left + right) // 2, bottom - 30, RIGHT_DOWN, RIGHT_UP)

    with glassboard():
        toggle(True)
        right_click_stays_inert(right_click_toolbar, "toolbar")
    return "Right-clicking the toolbar opened no browser menu."


def no_browser_menu_on_capture():
    with glassboard():
        press("win", "ctrl", "shift", "s")
        wait_for(lambda: visible(CAPTURE), "The global capture shortcut did not open the editor")
        time.sleep(1.5)
        right_click_stays_inert(lambda: click(MARK_BOX[0] + 60, MARK_BOX[1] + 60, RIGHT_DOWN, RIGHT_UP), "capture")
    return "Right-clicking the capture editor opened no browser menu."


def switcher_names():
    return powershell_uia(
        "$switcher=New-Object System.Windows.Automation.PropertyCondition($A::ClassNameProperty,'XamlExplorerHostIslandWindow');"
        "$w=$A::RootElement.FindFirst($T::Children,$switcher);"
        "if($w){foreach($e in $w.FindAll($T::Descendants,$Any)){$e.Current.Name}}"
    ).splitlines()


def require_left_out_of_switcher(name, names):
    (out / f"{name}-names.txt").write_text("\n".join(names))
    if FIXTURE not in names:
        raise Inconclusive(f"{name} did not list the fixture, so its names can't be trusted. See {name}-names.txt.")
    listed = [entry for entry in names if entry == TOOLBAR or entry.startswith(OVERLAY)]
    require(not listed, f"{name} lists Glassboard windows {listed}. See {name}.png.")


def left_out_of_alt_tab():
    with glassboard():
        toggle(True)
        draw_mark()
        user32.keybd_event(VK["alt"], 0, 0, 0)
        time.sleep(0.05)
        press("tab")
        time.sleep(1.5)
        screenshot("alt-tab")
        alt_tab = switcher_names()
        press("esc")
        user32.keybd_event(VK["alt"], 0, KEYUP, 0)
        require_left_out_of_switcher("alt-tab", alt_tab)
    return "Alt+Tab listed the app underneath and no Glassboard window."


def draws_on_another_virtual_desktop():
    with glassboard():
        press("ctrl", "win", "d")
        time.sleep(2)
        empty = screenshot("second-desktop-empty")
        try:
            toggle(True)
            drag(MARK_BOX[0] + 30, MARK_BOX[1] + 40, MARK_BOX[2] - 30, MARK_BOX[3] - 40)
            time.sleep(0.5)
            park_pointer()
            drawn = screenshot("second-desktop-drawing").crop(MARK_BOX)
            delta = changed_pixels(empty.crop(MARK_BOX), drawn)
            require(delta > 40, f"Drawing on a new virtual desktop changed only {delta} pixels")
            # A switch back to the first desktop would bring the whole fixture into view.
            require(delta < 15000, f"Drawing changed {delta} pixels, so Windows left the new desktop")
        finally:
            press("ctrl", "win", "f4")
            time.sleep(1.5)
    return "The shortcut drew on the current virtual desktop without switching away from it."


def tray_toggles_settings():
    with glassboard():
        center = find_tray_icon()
        click(*center)
        wait_for(lambda: visible(SETTINGS), "A tray click did not open Settings", timeout=10)
        time.sleep(2)
        require(visible(SETTINGS), "Settings closed by itself after a tray click")
        screenshot("settings-open")
        click(*center)
        time.sleep(3)
        screenshot("settings-after-second-click")
        require(not visible(SETTINGS), "A second tray click did not close Settings")
    return "The tray icon opened Settings, it stayed open, and a second click closed it."


def capture_excludes_drawing_and_copies():
    with glassboard():
        reference = screenshot("capture-reference")
        box = (MARK_BOX[0] + 20, MARK_BOX[1] + 20, MARK_BOX[2] - 20, MARK_BOX[3] - 20)
        toggle(True)
        draw_mark()
        press("win", "ctrl", "shift", "s")
        wait_for(lambda: visible(CAPTURE), "The global capture shortcut did not open the editor")
        time.sleep(1.5)
        drag(*box)
        time.sleep(0.5)
        screenshot("capture-editor")
        press("ctrl", "c")
        wait_for(lambda: not visible(CAPTURE), "Copy did not close the capture editor")
        copied = wait_for(lambda: ImageGrab.grabclipboard(), "The clipboard has no image after copying")
        require(isinstance(copied, Image.Image), f"The clipboard held {type(copied).__name__}, not an image")
        copied = copied.convert("RGB")
        copied.save(out / "capture-clipboard.png")
        expected = reference.crop(box)
        require(copied.size == expected.size, f"Copied {copied.size}, selected {expected.size}")
        delta = changed_pixels(expected, copied)
        require(delta < 20, f"{delta} copied pixels differ from the app underneath, so the drawing or an editor control leaked in")
    return "The clipboard image matched the app underneath, without the drawing."


def settings_save_survives_a_held_file():
    with glassboard():
        toggle(True)
        logged_before = log_text()
        handle = hold_preferences()
        try:
            press("ctrl", "3")
            time.sleep(0.5)
        finally:
            kernel32.CloseHandle(handle)
        time.sleep(2.5)
        screenshot("settings-save-after-hold")
        logged = log_text()[len(logged_before):]
        (out / "settings-save-log.txt").write_text(logged)
        saved = json.loads(PREFERENCES.read_text()).get("tool")
        require("Could not save preferences" not in logged, f"Showed a save error. Log: {logged.strip()}")
        require(saved == "rectangle", f"preferences.json still has tool {saved!r} after the file was released")
    return "A save during a 500 ms hold on preferences.json finished without an error."


def settings_load_survives_a_held_file():
    logged_before = log_text()
    with glassboard(held_at_launch=True, tool="rectangle"):
        logged = log_text()[len(logged_before):]
        (out / "settings-load-log.txt").write_text(logged)
        screenshot("settings-load-after-hold")
        require("Could not read" not in logged, f"Launching during a hold reset the settings. Log: {logged.strip()}")
        require(not visible(SETTINGS), "Launching during a hold opened Settings to report a problem")
        toggle(True)
        press("ctrl", "4")
        time.sleep(1.5)
        saved = json.loads(PREFERENCES.read_text())
        require(saved.get("colorMode") == "solid", f"The next save replaced the kept settings: {saved}")
    return "Launching during a 1 s hold on preferences.json kept the saved settings."


def log_text():
    return "".join(path.read_text(errors="replace") for path in sorted(LOG_DIR.glob("*.log")))


def uninstall_leaves_nothing():
    stop_glassboard()
    # An elevated runner writes the machine-wide key, but a normal user gets this entry.
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, RUN_KEY, 0, winreg.KEY_SET_VALUE) as key:
        winreg.SetValueEx(key, "Glassboard", 0, winreg.REG_SZ, f"{BINARY} ")
    subprocess.run([str(UNINSTALLER), "/S"], check=True, timeout=120)
    wait_for(lambda: not BINARY.exists(), f"The uninstaller left {BINARY}", timeout=60)
    # The silent uninstaller returns at once and keeps working in the background.
    deadline = time.monotonic() + 30
    while run_entries() and time.monotonic() < deadline:
        time.sleep(0.5)
    require(not run_entries(), f"The uninstaller left an Open at login entry: {run_entries()}")
    return "Uninstalling removed the app and a current-user Open at login entry."


def run_entries():
    entries = {}
    for root in (winreg.HKEY_CURRENT_USER, winreg.HKEY_LOCAL_MACHINE):
        with winreg.OpenKey(root, RUN_KEY) as key:
            for index in range(winreg.QueryInfoKey(key)[1]):
                name, data, _ = winreg.EnumValue(key, index)
                if "glassboard.exe" in str(data).lower():
                    entries[name] = data
    return entries


def open_at_login_round_trip():
    with glassboard():
        center = find_tray_icon()
        click(*center)
        wait_for(lambda: visible(SETTINGS), "A tray click did not open Settings", timeout=10)
        time.sleep(2)
        # Chromium builds its accessibility tree only after a client first asks for it.
        switch = None
        for _ in range(6):
            switch = uia_center(SETTINGS, "Open at login")
            if switch:
                break
            time.sleep(1)
        if not switch:
            raise Inconclusive("Settings did not expose the Open at login switch to UI Automation")
        click(*switch)
        time.sleep(1)
        screenshot("autostart-enabled")
        entries = run_entries()
        require(entries, "Turning on Open at login wrote no Run entry. See autostart-enabled.png.")
        click(*switch)
        time.sleep(1)
        screenshot("autostart-disabled")
        require(not run_entries(), f"Turning off Open at login left {run_entries()}")
    return f"Open at login wrote {entries} and turning it off removed it."


def main():
    check("installer", install)
    if not BINARY.exists():
        return
    start_fixture()
    # The tray check goes first: force-killing Glassboard leaves a stale icon behind.
    check("tray icon toggles Settings", tray_toggles_settings)
    check("draws over other apps, undo, click-through", draws_over_other_apps)
    check("toolbar stays above the focused drawing window", toolbar_stays_on_top)
    check("focus returns to the previous app", focus_returns_to_previous_app)
    check("left out of Alt+Tab", left_out_of_alt_tab)
    check("capture excludes the drawing and copies", capture_excludes_drawing_and_copies)
    check("settings save survives a held file", settings_save_survives_a_held_file)
    check("settings load survives a held file", settings_load_survives_a_held_file)
    for keys in (("f5",), ("ctrl", "r"), ("ctrl", "shift", "r"), ("ctrl", "p"), ("ctrl", "f"), ("f3",)):
        check(f"{'+'.join(keys)} leaves the drawing alone", browser_key_is_inert(*keys))
    check("no browser menu on the toolbar", no_browser_menu_on_toolbar)
    check("no browser menu on the capture editor", no_browser_menu_on_capture)
    check("Open at login turns on and off", open_at_login_round_trip)
    check("draws on another virtual desktop", draws_on_another_virtual_desktop)
    check("uninstall leaves nothing behind", uninstall_leaves_nothing)


try:
    main()
finally:
    for process in processes:
        process.kill()
    write_results()

summary = {status: sum(1 for result in results if result["status"] == status) for status in ("pass", "fail", "inconclusive")}
print(json.dumps(summary), flush=True)
sys.exit(1 if summary["fail"] or summary["inconclusive"] else 0)
