"""How Windows' own text engines group typing into steps of undo, asked of hidden controls of this script's own.

Each control is a child of a hidden window of this process; characters are sent to it as WM_CHAR messages and
undone with EM_UNDO (WM_UNDO for the classic edit). Nothing is sent to another program, and no input of the
system's (SendInput) is made.

- RICHEDIT50W (Msftedit.dll): Windows' own RichEdit, WordPad's, and the text services XAML's TextBox and
  RichEditBox (UWP, WinUI) are built on.
- RICHEDIT60W: Office's RichEdit, which Windows 11's Notepad ships in its package (Notepad\riched20.dll) and
  types into. The package's folder does not let another program load it: copy the DLL (and msptls.dll beside it)
  out, and give its path as the first argument.
- EDIT: the classic edit control (Windows 10's Notepad, every Win32 text field). Its undo is of one level, a
  toggle; without the focus of the system, which a hidden window has not, it takes back one character.

python -I richedit_undo.py [path\to\riched20.dll]
"""

import ctypes
import sys
import time
from ctypes import wintypes as w

user32 = ctypes.WinDLL("user32", use_last_error=True)
kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)

LRESULT = ctypes.c_ssize_t
user32.CreateWindowExW.restype = w.HWND
user32.CreateWindowExW.argtypes = [w.DWORD, w.LPCWSTR, w.LPCWSTR, w.DWORD, ctypes.c_int, ctypes.c_int, ctypes.c_int,
                                   ctypes.c_int, w.HWND, w.HMENU, w.HINSTANCE, w.LPVOID]
user32.SendMessageW.restype = LRESULT
user32.SendMessageW.argtypes = [w.HWND, w.UINT, w.WPARAM, w.LPARAM]
user32.DestroyWindow.argtypes = [w.HWND]
kernel32.LoadLibraryW.restype = w.HMODULE
kernel32.LoadLibraryW.argtypes = [w.LPCWSTR]

WS_CHILD = 0x40000000
WS_POPUP = 0x80000000
ES_MULTILINE = 0x0004
WM_CHAR = 0x0102
WM_KEYDOWN = 0x0100
WM_KEYUP = 0x0101
WM_GETTEXT = 0x000D
WM_GETTEXTLENGTH = 0x000E
WM_UNDO = 0x0304
EM_UNDO = 0x00C7
EM_CANUNDO = 0x00C6
EM_SETSEL = 0x00B1
WM_SETFOCUS = 0x0007
VK_BACK = 0x08
VK_LEFT = 0x25
VK_RIGHT = 0x27


def text(hwnd):
    n = user32.SendMessageW(hwnd, WM_GETTEXTLENGTH, 0, 0)
    buf = ctypes.create_unicode_buffer(n + 1)
    user32.SendMessageW(hwnd, WM_GETTEXT, n + 1, ctypes.addressof(buf))
    return buf.value


def typed(hwnd, s, pause=0.0):
    for ch in s:
        user32.SendMessageW(hwnd, WM_CHAR, ord(ch), 1)
        if pause:
            time.sleep(pause)


def key(hwnd, vk):
    user32.SendMessageW(hwnd, WM_KEYDOWN, vk, 1)
    user32.SendMessageW(hwnd, WM_KEYUP, vk, 0xC0000001)


def undo_all(hwnd, undo_msg, limit=8):
    seen = [text(hwnd)]
    for _ in range(limit):
        if not user32.SendMessageW(hwnd, EM_CANUNDO, 0, 0):
            break
        user32.SendMessageW(hwnd, undo_msg, 0, 0)
        now = text(hwnd)
        seen.append(now)
        if now == "":
            break
    return seen


EM_SETTEXTMODE = 0x0459
TM_PLAINTEXT = 1
TM_MULTILEVELUNDO = 8


def control(cls, parent, plain=False):
    hwnd = user32.CreateWindowExW(0, cls, "", WS_CHILD | ES_MULTILINE, 0, 0, 400, 200, parent, None, None, None)
    assert hwnd, ctypes.get_last_error()
    if plain:
        user32.SendMessageW(hwnd, EM_SETTEXTMODE, TM_PLAINTEXT | TM_MULTILEVELUNDO, 0)
    user32.SendMessageW(hwnd, WM_SETFOCUS, 0, 0)
    return hwnd


def case(cls, undo_msg, name, act, plain=False):
    parent = user32.CreateWindowExW(0, "STATIC", "lurq undo probe", WS_POPUP, 0, 0, 400, 200, None, None, None, None)
    hwnd = control(cls, parent, plain)
    try:
        act(hwnd)
        print(f"  {name}: {undo_all(hwnd, undo_msg)!r}")
    finally:
        user32.DestroyWindow(parent)


def probe(cls, undo_msg, plain=False):
    print(cls + (" (plain text)" if plain else ""))
    case = lambda *a: globals()["case"](*a, plain=plain)
    case(cls, undo_msg, "typed 'hello big world'", lambda h: typed(h, "hello big world"))
    case(cls, undo_msg, "typed 'hello  big' (two spaces)", lambda h: typed(h, "hello  big"))
    case(cls, undo_msg, "typed 'one,two.three'", lambda h: typed(h, "one,two.three"))
    case(cls, undo_msg, "typed 'abc', 1.5 s, 'def'", lambda h: (typed(h, "abc"), time.sleep(1.5), typed(h, "def")))
    case(cls, undo_msg, "typed 'hello world', 3 backspaces", lambda h: (typed(h, "hello world"), [key(h, VK_BACK) for _ in range(3)]))
    case(cls, undo_msg, "typed 'abc', left, 'X'", lambda h: (typed(h, "abc"), key(h, VK_LEFT), typed(h, "X")))


if __name__ == "__main__":
    assert kernel32.LoadLibraryW("Msftedit.dll"), "Msftedit.dll"
    probe("RICHEDIT50W", EM_UNDO)
    if len(sys.argv) > 1:
        # Windows 11's Notepad ships Office's RichEdit (riched20.dll in its package, class RICHEDIT60W), not
        # msftedit's; it is loaded from the package as it is and asked the same, in plain text as Notepad is.
        assert kernel32.LoadLibraryW(sys.argv[1]), (sys.argv[1], ctypes.get_last_error())
        probe("RICHEDIT60W", EM_UNDO, plain=True)
    probe("EDIT", WM_UNDO)
    sys.stdout.flush()
