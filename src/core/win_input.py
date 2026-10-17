import ctypes
from ctypes import wintypes
import time

# Win32 Constants
INPUT_MOUSE = 0
INPUT_KEYBOARD = 1
INPUT_HARDWARE = 2

# Mouse flags
MOUSEEVENTF_MOVE = 0x0001
MOUSEEVENTF_LEFTDOWN = 0x0002
MOUSEEVENTF_LEFTUP = 0x0004
MOUSEEVENTF_RIGHTDOWN = 0x0008
MOUSEEVENTF_RIGHTUP = 0x0010
MOUSEEVENTF_MIDDLEDOWN = 0x0020
MOUSEEVENTF_MIDDLEUP = 0x0040
MOUSEEVENTF_XDOWN = 0x0080
MOUSEEVENTF_XUP = 0x0100
MOUSEEVENTF_WHEEL = 0x0800
MOUSEEVENTF_HWHEEL = 0x1000
MOUSEEVENTF_ABSOLUTE = 0x8000

# XButton Data
XBUTTON1 = 0x0001
XBUTTON2 = 0x0002

# Keyboard flags
KEYEVENTF_KEYDOWN = 0x0000
KEYEVENTF_EXTENDEDKEY = 0x0001
KEYEVENTF_KEYUP = 0x0002
KEYEVENTF_UNICODE = 0x0004
KEYEVENTF_SCANCODE = 0x0008

# CTypes Structures
class MOUSEINPUT(ctypes.Structure):
    _fields_ = (
        ("dx", wintypes.LONG),
        ("dy", wintypes.LONG),
        ("mouseData", wintypes.DWORD),
        ("dwFlags", wintypes.DWORD),
        ("time", wintypes.DWORD),
        ("dwExtraInfo", ctypes.c_size_t),
    )

class KEYBDINPUT(ctypes.Structure):
    _fields_ = (
        ("wVk", wintypes.WORD),
        ("wScan", wintypes.WORD),
        ("dwFlags", wintypes.DWORD),
        ("time", wintypes.DWORD),
        ("dwExtraInfo", ctypes.c_size_t),
    )

class HARDWAREINPUT(ctypes.Structure):
    _fields_ = (
        ("uMsg", wintypes.DWORD),
        ("wParamL", wintypes.WORD),
        ("wParamH", wintypes.WORD),
    )

class _INPUTunion(ctypes.Union):
    _fields_ = (
        ("mi", MOUSEINPUT),
        ("ki", KEYBDINPUT),
        ("hi", HARDWAREINPUT),
    )

class INPUT(ctypes.Structure):
    _fields_ = (
        ("type", wintypes.DWORD),
        ("union", _INPUTunion),
    )

LPINPUT = ctypes.POINTER(INPUT)
SendInput = ctypes.windll.user32.SendInput
SendInput.argtypes = (wintypes.UINT, LPINPUT, ctypes.c_int)
SendInput.restype = wintypes.UINT

MapVirtualKeyW = ctypes.windll.user32.MapVirtualKeyW
VkKeyScanW = ctypes.windll.user32.VkKeyScanW
GetCursorPos = ctypes.windll.user32.GetCursorPos
SetCursorPos = ctypes.windll.user32.SetCursorPos

class POINT(ctypes.Structure):
    _fields_ = [("x", ctypes.c_long), ("y", ctypes.c_long)]

def get_mouse_position() -> tuple[int, int]:
    pt = POINT()
    GetCursorPos(ctypes.byref(pt))
    return pt.x, pt.y

def set_mouse_position(x: int, y: int):
    SetCursorPos(int(x), int(y))

def _send_input(inputs: list[INPUT]):
    if not inputs:
        return
    n_inputs = len(inputs)
    arr = (INPUT * n_inputs)(*inputs)
    SendInput(n_inputs, arr, ctypes.sizeof(INPUT))

def mouse_down(button: str = "left"):
    button = button.lower()
    inp = INPUT(type=INPUT_MOUSE)
    
    if button == "left":
        inp.union.mi.dwFlags = MOUSEEVENTF_LEFTDOWN
    elif button == "right":
        inp.union.mi.dwFlags = MOUSEEVENTF_RIGHTDOWN
    elif button in ("middle", "mid"):
        inp.union.mi.dwFlags = MOUSEEVENTF_MIDDLEDOWN
    elif button in ("x1", "side1", "back"):
        inp.union.mi.dwFlags = MOUSEEVENTF_XDOWN
        inp.union.mi.mouseData = XBUTTON1
    elif button in ("x2", "side2", "forward"):
        inp.union.mi.dwFlags = MOUSEEVENTF_XDOWN
        inp.union.mi.mouseData = XBUTTON2
    else:
        inp.union.mi.dwFlags = MOUSEEVENTF_LEFTDOWN
        
    _send_input([inp])

def mouse_up(button: str = "left"):
    button = button.lower()
    inp = INPUT(type=INPUT_MOUSE)
    
    if button == "left":
        inp.union.mi.dwFlags = MOUSEEVENTF_LEFTUP
    elif button == "right":
        inp.union.mi.dwFlags = MOUSEEVENTF_RIGHTUP
    elif button in ("middle", "mid"):
        inp.union.mi.dwFlags = MOUSEEVENTF_MIDDLEUP
    elif button in ("x1", "side1", "back"):
        inp.union.mi.dwFlags = MOUSEEVENTF_XUP
        inp.union.mi.mouseData = XBUTTON1
    elif button in ("x2", "side2", "forward"):
        inp.union.mi.dwFlags = MOUSEEVENTF_XUP
        inp.union.mi.mouseData = XBUTTON2
    else:
        inp.union.mi.dwFlags = MOUSEEVENTF_LEFTUP
        
    _send_input([inp])

def mouse_click(button: str = "left", hold_time: float = 0.001):
    mouse_down(button)
    if hold_time > 0:
        time.sleep(hold_time)
    mouse_up(button)

def mouse_double_click(button: str = "left", delay_between: float = 0.05):
    mouse_click(button)
    time.sleep(delay_between)
    mouse_click(button)

def mouse_triple_click(button: str = "left", delay_between: float = 0.05):
    mouse_click(button)
    time.sleep(delay_between)
    mouse_click(button)
    time.sleep(delay_between)
    mouse_click(button)

def mouse_scroll_vertical(amount: int):
    # amount: 120 = 1 standard wheel notch up, -120 = 1 standard wheel notch down
    inp = INPUT(type=INPUT_MOUSE)
    inp.union.mi.dwFlags = MOUSEEVENTF_WHEEL
    inp.union.mi.mouseData = amount
    _send_input([inp])

def mouse_scroll_horizontal(amount: int):
    # amount: 120 = right, -120 = left
    inp = INPUT(type=INPUT_MOUSE)
    inp.union.mi.dwFlags = MOUSEEVENTF_HWHEEL
    inp.union.mi.mouseData = amount
    _send_input([inp])

# Virtual key map for special names
SPECIAL_VK = {
    "space": 0x20,
    "enter": 0x0D,
    "return": 0x0D,
    "tab": 0x09,
    "escape": 0x1B,
    "esc": 0x1B,
    "backspace": 0x08,
    "shift": 0x10,
    "ctrl": 0x11,
    "control": 0x11,
    "alt": 0x12,
    "capslock": 0x14,
    "pageup": 0x21,
    "pagedown": 0x22,
    "end": 0x23,
    "home": 0x24,
    "left": 0x25,
    "up": 0x26,
    "right": 0x27,
    "down": 0x28,
    "insert": 0x2D,
    "delete": 0x2E,
    "f1": 0x70,
    "f2": 0x71,
    "f3": 0x72,
    "f4": 0x73,
    "f5": 0x74,
    "f6": 0x75,
    "f7": 0x76,
    "f8": 0x77,
    "f9": 0x78,
    "f10": 0x79,
    "f11": 0x7A,
    "f12": 0x7B,
    "numpad0": 0x60,
    "numpad1": 0x61,
    "numpad2": 0x62,
    "numpad3": 0x63,
    "numpad4": 0x64,
    "numpad5": 0x65,
    "numpad6": 0x66,
    "numpad7": 0x67,
    "numpad8": 0x68,
    "numpad9": 0x69,
}

def _get_vk_code(key: str) -> tuple[int, int]:
    """Returns (vk_code, scan_code)"""
    key_lower = key.strip().lower()
    if key_lower in SPECIAL_VK:
        vk = SPECIAL_VK[key_lower]
        scan = MapVirtualKeyW(vk, 0)
        return vk, scan
    
    if len(key) == 1:
        vkey_scan = VkKeyScanW(ord(key))
        vk = vkey_scan & 0xFF
        if vk != 0xFF:
            scan = MapVirtualKeyW(vk, 0)
            return vk, scan
        
    return 0, 0

def key_down(key: str):
    vk, scan = _get_vk_code(key)
    if vk != 0:
        inp = INPUT(type=INPUT_KEYBOARD)
        inp.union.ki.wVk = vk
        inp.union.ki.wScan = scan
        inp.union.ki.dwFlags = KEYEVENTF_KEYDOWN
        _send_input([inp])
    else:
        # Fallback to unicode
        for char in key:
            inp = INPUT(type=INPUT_KEYBOARD)
            inp.union.ki.wVk = 0
            inp.union.ki.wScan = ord(char)
            inp.union.ki.dwFlags = KEYEVENTF_UNICODE
            _send_input([inp])

def key_up(key: str):
    vk, scan = _get_vk_code(key)
    if vk != 0:
        inp = INPUT(type=INPUT_KEYBOARD)
        inp.union.ki.wVk = vk
        inp.union.ki.wScan = scan
        inp.union.ki.dwFlags = KEYEVENTF_KEYUP
        _send_input([inp])
    else:
        # Unicode up
        for char in key:
            inp = INPUT(type=INPUT_KEYBOARD)
            inp.union.ki.wVk = 0
            inp.union.ki.wScan = ord(char)
            inp.union.ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
            _send_input([inp])

def key_press(key: str, hold_time: float = 0.005):
    key_down(key)
    if hold_time > 0:
        time.sleep(hold_time)
    key_up(key)

def key_combo(keys: list[str], hold_time: float = 0.01):
    """Press multiple keys down sequentially, hold, then release in reverse order."""
    for k in keys:
        key_down(k)
        time.sleep(0.002)
    if hold_time > 0:
        time.sleep(hold_time)
    for k in reversed(keys):
        key_up(k)
        time.sleep(0.002)

def send_text_unicode(text: str, char_delay: float = 0.01):
    for char in text:
        inp_down = INPUT(type=INPUT_KEYBOARD)
        inp_down.union.ki.wVk = 0
        inp_down.union.ki.wScan = ord(char)
        inp_down.union.ki.dwFlags = KEYEVENTF_UNICODE
        
        inp_up = INPUT(type=INPUT_KEYBOARD)
        inp_up.union.ki.wVk = 0
        inp_up.union.ki.wScan = ord(char)
        inp_up.union.ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
        
        _send_input([inp_down, inp_up])
        if char_delay > 0:
            time.sleep(char_delay)
