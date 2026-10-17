#!/usr/bin/env python3
"""Unix PTY smoke checks for real example binaries; standard library only.

Build with `cargo build --workspace --examples` first. Checks exit codes,
terminal restoration, input during background work, resize and Linux idle ticks.
"""
import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time

ROOT = Path(__file__).resolve().parents[1]

def smoke(name, arguments, interactions, idle=False):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
    original = termios.tcgetattr(slave)
    def connect():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)
    process = subprocess.Popen([str(ROOT / "target/debug/examples" / name), *arguments],
        stdin=slave, stdout=slave, stderr=slave, preexec_fn=connect,
        env={**os.environ, "TERM": "xterm-256color"})
    output = bytearray()
    def drain(duration):
        until = time.monotonic() + duration
        while time.monotonic() < until:
            if select.select([master], [], [], max(0, until - time.monotonic()))[0]:
                try: output.extend(os.read(master, 65536))
                except OSError: break
    try:
        drain(.15)
        if idle:
            stat = Path(f"/proc/{process.pid}/stat")
            if stat.exists():
                def ticks():
                    fields = stat.read_text().split()
                    return int(fields[13]) + int(fields[14])
                before = ticks()
                drain(.4)
                delta = ticks() - before
                assert delta <= 1, f"{name}: {delta} CPU ticks in 400ms idle"
                print(f"{name}: idle CPU ticks={delta}/400ms")
        for keys in interactions:
            if keys is None:
                fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 18, 60, 0, 0))
                os.kill(process.pid, signal.SIGWINCH)
            else:
                os.write(master, keys)
            drain(.12)
        drain(.1)
        process.wait(timeout=3)
        assert process.returncode == 0, f"{name}: exit {process.returncode}: {output[-1000:]!r}"
        assert termios.tcgetattr(slave) == original, f"{name}: terminal not restored"
        assert b"panicked" not in output and b"Error:" not in output, output[-1000:]
        if name == "terratui-widgets":
            assert b"%" in output and b"Profile" in output, "missing progress/dialog output"
        print(f"{name}: keyboard, resize, exit and terminal restoration passed")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)

F1, F3, F5, F6, F10 = b"\x1bOP", b"\x1bOR", b"\x1b[15~", b"\x1b[17~", b"\x1b[21~"
smoke("terrabox-demo", [], [b" ", b"z", None, b"\x1b"])
smoke("terratui-desktop", [], [b"\t", b"\x1b[Z", F10, b"\x1b[C", b"\r", b"X", b"\t", b"\r", F1, b"\r", None, b"\x11"], idle=True)
with tempfile.TemporaryDirectory(prefix="terrabox-smoke-") as folder:
    base = Path(folder)
    (base / "Documents").mkdir()
    for i in range(40): (base / f"sample-{i:02}.txt").write_text("example\n")
    smoke("terratui-files", [folder], [b"\t", b"\x1b[B", b"\r", b"\x7f", b"\x1b[F", b"\x1b[5~", F3, b"\r", None, F10], idle=True)
smoke("terratui-widgets", [], [F5, b"\t", b"\t", F1, "λ".encode(), b"\t", b"\x1b[Z", b"\r", F6, None, b"\x11"], idle=True)
