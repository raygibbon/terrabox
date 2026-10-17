#!/usr/bin/env python3
"""Capture actual xterm windows (X11/Xwayland, xterm and ImageMagick required).

Run `cargo build --workspace --examples` first. Screenshots use a temporary,
non-personal directory fixture. Review all images before committing them.
"""
import ctypes as C
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets/screenshots"

# Send actual X key events only to the emulator windows created by this script.
x = C.CDLL("libX11.so.6")
x.XOpenDisplay.restype = C.c_void_p
x.XStringToKeysym.argtypes = [C.c_char_p]
x.XStringToKeysym.restype = C.c_ulong
x.XKeysymToKeycode.argtypes = [C.c_void_p, C.c_ulong]
x.XKeysymToKeycode.restype = C.c_uint

class KeyEvent(C.Structure):
    _fields_ = [("type", C.c_int), ("serial", C.c_ulong), ("send_event", C.c_int),
                ("display", C.c_void_p), ("window", C.c_ulong), ("root", C.c_ulong),
                ("subwindow", C.c_ulong), ("time", C.c_ulong), ("x", C.c_int),
                ("y", C.c_int), ("x_root", C.c_int), ("y_root", C.c_int),
                ("state", C.c_uint), ("keycode", C.c_uint), ("same_screen", C.c_int)]

class XEvent(C.Union):
    _fields_ = [("key", KeyEvent), ("pad", C.c_long * 24)]

x.XSendEvent.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_long, C.POINTER(XEvent)]
x.XFlush.argtypes = [C.c_void_p]
x.XCloseDisplay.argtypes = [C.c_void_p]
DISPLAY = x.XOpenDisplay(None)
if not DISPLAY:
    raise SystemExit("An X11/Xwayland display is required; no images generated")

def key(window, name):
    state = 4 if name.startswith("Ctrl+") else 0
    name = name.removeprefix("Ctrl+")
    code = x.XKeysymToKeycode(DISPLAY, x.XStringToKeysym(name.encode()))
    for kind, mask in [(2, 1), (3, 2)]:
        event = XEvent()
        event.key = KeyEvent(type=kind, send_event=1, display=DISPLAY, window=window,
                             state=state, keycode=code, same_screen=1)
        x.XSendEvent(DISPLAY, window, 1, mask, C.byref(event))
    x.XFlush(DISPLAY)
    time.sleep(.15)

def capture(name, args=(), actions=(), extra=None, quit_key="Ctrl+q"):
    title = f"TerraBox capture {os.getpid()} {name}"
    proc = subprocess.Popen(["xterm", "-xrm", "*allowSendEvents:true",
        "-xrm", "*color4:#163456", "-xrm", "*color6:#36a6a6",
        "-geometry", "100x30", "-fa", "DejaVu Sans Mono", "-fs", "12",
        "-T", title, "-e", str(ROOT / "target/debug/examples" / name), *map(str, args)])
    try:
        for _ in range(40):
            tree = subprocess.check_output(["xwininfo", "-root", "-tree"], text=True)
            match = re.search(r'(0x[0-9a-f]+) "' + re.escape(title) + r'"', tree)
            if match:
                window = int(match[1], 16)
                # The VT widget is the child of xterm's top-level shell.
                info = subprocess.check_output(["xwininfo", "-id", hex(window), "-tree"], text=True)
                children = re.findall(r'^\s+(0x[0-9a-f]+) \(has no name\)', info, re.M)
                target = int(children[-1], 16) if children else window
                break
            if proc.poll() is not None:
                raise RuntimeError(f"{name} exited before capture")
            time.sleep(.1)
        else:
            raise RuntimeError("xterm did not create a window")
        time.sleep(.3)
        for action in actions:
            if isinstance(action, float): time.sleep(action)
            else: key(target, action)
        subprocess.run(["import", "-window", hex(window), str(OUT / f"{name}.png")], check=True)
        if extra:
            key(target, "F1")
            subprocess.run(["import", "-window", hex(window), str(OUT / extra)], check=True)
            key(target, "Escape")
        if name == "terratui-desktop": key(target, "Escape")
        key(target, quit_key)
        if proc.wait(timeout=5) != 0: raise RuntimeError(f"{name} exited unsuccessfully")
        print(f"Captured {name}")
    finally:
        if proc.poll() is None:
            proc.terminate()
            proc.wait(timeout=5)

try:
    OUT.mkdir(parents=True, exist_ok=True)
    capture("terratui-desktop", actions=("F10",))
    with tempfile.TemporaryDirectory(prefix="terrabox-demo-") as folder:
        fixture = Path(folder)
        for directory in ["Documents", "Examples", "Projects"]: (fixture / directory).mkdir()
        for i in range(1, 27): (fixture / f"sample-{i:02}.txt").write_text(f"Sample document {i}\n")
        capture("terratui-files", args=(fixture,), actions=("Tab", "Down", "Down"), quit_key="F10")
    capture("terratui-widgets", actions=("F5", .8, "Tab", "Tab"), extra="terratui-dialog.png")
    capture("terrabox-demo", actions=(.5, "space"), quit_key="Escape")
finally:
    x.XCloseDisplay(DISPLAY)
