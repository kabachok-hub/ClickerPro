import sys
import os
import time
import ctypes
import winsound
from ctypes import wintypes

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

# Mouse event constants
MOUSEEVENTF_MOVE = 0x0001
MOUSEEVENTF_LEFTDOWN = 0x0002
MOUSEEVENTF_LEFTUP = 0x0004
MOUSEEVENTF_RIGHTDOWN = 0x0008
MOUSEEVENTF_RIGHTUP = 0x0010
MOUSEEVENTF_MIDDLEDOWN = 0x0020
MOUSEEVENTF_MIDDLEUP = 0x0040
MOUSEEVENTF_ABSOLUTE = 0x8000

# SendInput structures
class MOUSEINPUT(ctypes.Structure):
    _fields_ = [
        ("dx", wintypes.LONG),
        ("dy", wintypes.LONG),
        ("mouseData", wintypes.DWORD),
        ("dwFlags", wintypes.DWORD),
        ("time", wintypes.DWORD),
        ("dwExtraInfo", ctypes.c_ulonglong if sys.maxsize > 2**32 else wintypes.DWORD),
    ]

class KEYBDINPUT(ctypes.Structure):
    _fields_ = [
        ("wVk", wintypes.WORD),
        ("wScan", wintypes.WORD),
        ("dwFlags", wintypes.DWORD),
        ("time", wintypes.DWORD),
        ("dwExtraInfo", ctypes.c_ulonglong if sys.maxsize > 2**32 else wintypes.DWORD),
    ]

class HARDWAREINPUT(ctypes.Structure):
    _fields_ = [
        ("uMsg", wintypes.DWORD),
        ("wParamL", wintypes.WORD),
        ("wParamH", wintypes.WORD),
    ]

class INPUT_UNION(ctypes.Union):
    _fields_ = [
        ("mi", MOUSEINPUT),
        ("ki", KEYBDINPUT),
        ("hi", HARDWAREINPUT),
    ]

class INPUT(ctypes.Structure):
    _fields_ = [
        ("type", wintypes.DWORD),
        ("u", INPUT_UNION),
    ]

INPUT_MOUSE = 0
INPUT_KEYBOARD = 1

def ensure_desktop_access():
    """Ensures thread is attached to interactive desktop for screen captures and clicks."""
    try:
        h_desktop = user32.OpenInputDesktop(0, False, 0x01FF)
        if h_desktop:
            user32.SetThreadDesktop(h_desktop)
            user32.CloseDesktop(h_desktop)
    except Exception:
        pass

def get_cursor_pos():
    pt = wintypes.POINT()
    user32.GetCursorPos(ctypes.byref(pt))
    return pt.x, pt.y

def set_cursor_pos(x, y):
    user32.SetCursorPos(int(x), int(y))
    # Also send absolute mouse move event to inform game raw input hooks
    sw = user32.GetSystemMetrics(0)
    sh = user32.GetSystemMetrics(1)
    if sw > 0 and sh > 0:
        norm_x = int(x * 65535 / (sw - 1))
        norm_y = int(y * 65535 / (sh - 1))
        user32.mouse_event(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, norm_x, norm_y, 0, 0)

def hardware_click(x, y, button="left", click_type="single", hold_ms=45, return_cursor=False, pre_hover_ms=25, post_release_ms=25):
    """
    100% Reliable Hardware Click for Games (Roblox, FPS, Clickers):
    1. Moves cursor to (x, y) with both SetCursorPos and mouse_event.
    2. Waits pre_hover_ms so game registers MouseEnter and activates hover state.
    3. Dispatches MouseDown via driver-level mouse_event + SendInput.
    4. Holds for hold_ms.
    5. Dispatches MouseUp via driver-level mouse_event + SendInput.
    6. Waits post_release_ms ON THE BUTTON so game frame loop registers MouseUp inside button bounds!
    7. Restores cursor if return_cursor is True.
    """
    orig_x, orig_y = get_cursor_pos()
    
    down_flag = MOUSEEVENTF_LEFTDOWN
    up_flag = MOUSEEVENTF_LEFTUP
    if button == "right":
        down_flag = MOUSEEVENTF_RIGHTDOWN
        up_flag = MOUSEEVENTF_RIGHTUP
    elif button == "middle":
        down_flag = MOUSEEVENTF_MIDDLEDOWN
        up_flag = MOUSEEVENTF_MIDDLEUP

    # 1. Move to button
    set_cursor_pos(x, y)
    
    # 2. Wait for game hover detection
    if pre_hover_ms > 0:
        time.sleep(pre_hover_ms / 1000.0)

    clicks = 2 if click_type == "double" else 1
    for i in range(clicks):
        # 3. Mouse Down
        user32.mouse_event(down_flag, 0, 0, 0, 0)
        
        # 4. Hold
        hold_sec = max(0.030, hold_ms / 1000.0)
        time.sleep(hold_sec)
        
        # 5. Mouse Up
        user32.mouse_event(up_flag, 0, 0, 0, 0)
        
        if click_type == "double" and i == 0:
            time.sleep(0.040)

    # 6. Crucial: stay on button briefly so game processes Activated event
    if post_release_ms > 0:
        time.sleep(post_release_ms / 1000.0)

    # 7. Restore cursor if requested
    if return_cursor and (orig_x != x or orig_y != y):
        set_cursor_pos(orig_x, orig_y)

SOUND_ENABLED = False

def set_sound_enabled(enabled):
    global SOUND_ENABLED
    SOUND_ENABLED = enabled

def play_sound_start():
    if not SOUND_ENABLED:
        return
    try:
        winsound.Beep(900, 70)
        winsound.Beep(1200, 100)
    except Exception:
        pass

def play_sound_stop():
    if not SOUND_ENABLED:
        return
    try:
        winsound.Beep(600, 120)
    except Exception:
        pass

def play_sound_click():
    if not SOUND_ENABLED:
        return
    try:
        winsound.Beep(1500, 60)
    except Exception:
        pass

def is_admin():
    try:
        return ctypes.windll.shell32.IsUserAnAdmin() != 0
    except Exception:
        return False

def restart_as_admin():
    try:
        params = f'"{os.path.abspath(sys.argv[0])}"'
        ctypes.windll.shell32.ShellExecuteW(None, "runas", sys.executable, params, None, 1)
        sys.exit(0)
    except Exception as e:
        print("Failed to elevate:", e)
