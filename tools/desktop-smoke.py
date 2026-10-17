"""Exercise the TerraTUI demo in a real Linux PTY after building its example."""

import errno
import fcntl
import os
from pathlib import Path
import select
import signal
import struct
import subprocess
import sys
import termios
import time


def main():
    if sys.platform != "linux":
        raise SystemExit("This PTY smoke test requires Linux")
    root = Path(__file__).resolve().parent.parent
    binary = Path(sys.argv[1]) if len(sys.argv) > 1 else root / "target/debug/examples/desktop"
    master, slave = os.openpty()
    original = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))

    def acquire_terminal():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    process = subprocess.Popen(
        [str(binary)], stdin=slave, stdout=slave, stderr=slave,
        env={**os.environ, "TERM": "xterm"}, preexec_fn=acquire_terminal,
    )

    def read_output(timeout):
        output = bytearray()
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            ready, _, _ = select.select([master], [], [], max(0, deadline - time.monotonic()))
            if not ready:
                break
            try:
                chunk = os.read(master, 65536)
            except OSError as error:
                if error.errno == errno.EIO:
                    break
                raise
            if not chunk:
                break
            output.extend(chunk)
        return bytes(output)

    def expect(text):
        output = bytearray()
        deadline = time.monotonic() + 5
        while text not in output and time.monotonic() < deadline:
            output.extend(read_output(0.05))
        assert text in output, (text, bytes(output))
        return bytes(output)

    try:
        expect(b"Navigation")
        read_output(0.1)
        assert not read_output(0.15), "idle application emitted unsolicited frames"
        os.write(master, b"\t")
        assert read_output(0.15), "focus switch did not redraw"
        os.write(master, b"\x1bOP")
        expect(b"retained terminal application")
        os.write(master, b"\x11")
        read_output(0.1)
        assert process.poll() is None, "modal allowed the global quit command"
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 12, 50, 0, 0))
        os.kill(process.pid, signal.SIGWINCH)
        assert read_output(0.2), "resize did not redraw"
        os.write(master, b"\x1b")
        assert read_output(0.15), "Escape did not close the modal"
        os.write(master, b"\x1bOP")
        expect(b"retained terminal application")
        os.write(master, b"\r")
        assert read_output(0.15), "Enter did not close the modal"
        os.write(master, b"\x1b[21~\x1b[C\r")
        process.wait(timeout=5)
        assert process.returncode == 0, process.returncode
        read_output(0.1)
        assert termios.tcgetattr(slave) == original, "terminal attributes were not restored"
        print("PASS: idle, focus, modal capture, resize, menu quit, terminal restoration")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)


if __name__ == "__main__":
    main()
