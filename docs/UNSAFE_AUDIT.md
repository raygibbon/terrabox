# Unsafe Rust audit

The library exposes a safe public API. Unsafe code is confined to `src/platform/unix.rs` and `src/platform/windows.rs`; the input parser, terminfo parser, and renderer use safe Rust.

## Unix

`libc` calls receive valid file descriptors and initialized Rust buffers. `MaybeUninit` values are read only after successful `tcgetattr`, `ioctl`, or `sigaction` calls. The resize self-pipe descriptors are created together, checked during setup, and closed in `Drop`; the backend is the sole owner. Borrowed event descriptors cannot outlive the backend. The SIGWINCH callback loads one atomic descriptor and performs a one-byte `write`, which is async-signal-safe. The handler preserves thread-local errno. The atomic is set to `-1` before the pipe is closed, and teardown waits for in-flight handlers before closing descriptors so they cannot write to a reused descriptor. A RAII lease releases process ownership exactly once after all descriptor cleanup, including failed initialization. Only one terminal can own the process signal handler at a time.

`from_stdio()` borrows process descriptors; callers must keep them open while the terminal exists. `new()` and `from_fds()` own their terminal descriptors. The terminal mode is restored before owned descriptors are dropped. Reads, writes, and termios changes retry EINTR. Partial suspend failures retain an outstanding-cleanup flag for drop/resume retries; the terminal is considered suspended during partial restoration. Mouse configuration while suspended is deferred until resume.

## Windows

Console handles are checked before use and duplicated into Rust `OwnedHandle`s. The process standard handles are never closed by the backend. Raw handle aliases stay valid through backend cleanup; the owned duplicates close afterward. A process-local atomic lease prevents overlapping terminal lifetimes from saving and restoring conflicting console state, including while suspended. Failed construction releases the lease.

Win32 calls receive initialized records or writable output pointers, and return values are checked. Input record union fields are read only after their event type is checked. UTF-16 surrogate conversion checks ranges before arithmetic and never creates an invalid Rust `char`. Key repeats are drained without allocating an event per repeat. Console writes handle partial writes and zero-byte writes, and chunks of valid UTF-8 end at character boundaries.

Initialization saves input/output modes, the output code page, and cursor information before mutation. The input code page is not changed: native W input already supplies UTF-16. Every setup error attempts restoration. VT cleanup is emitted only after VT output was enabled. Restoration attempts every saved setting and captures each failure immediately, retaining the first restoration error. A failed output write is returned ahead of restoration errors. Active state and outstanding cleanup are separate: a partially restored terminal is suspended, and `Drop` or a subsequent resume retries remaining restoration. `Drop` does not panic. Screen cleanup after a failed console write remains best effort.

The static `extern "system"` control callback consumes only `CTRL_BREAK_EVENT`, without allocating, blocking, panicking or accessing console state. Its registration is owned by the active backend and removed on suspend/drop/error cleanup. Ctrl+C is native key input because processed input is disabled. Existing application handlers are not removed. Test builds add an atomic signal-delivery counter; production builds do not. External code must not attach/detach the console while a terminal exists, because Windows resets registered handlers in that case.

Construction uses small unsafe blocks for standard-handle queries, mode/cursor queries and temporary borrowed handles used for duplication. Runtime unsafe blocks are limited to synchronous Win32 calls or discriminator-checked union reads. Buffer lengths are bounded by their Rust allocations; partial writes and zero writes are handled. Native records become safe Rust events immediately after translation. Test-only failure checkpoints cover handles, output code page, input/output modes, mouse, alternate-screen entry and cursor setup. Twenty create/drop cycles with three suspend/resume cycles each leave the process handle count unchanged.

Tests use `CREATE_NEW_CONSOLE`, verify attachment via `GetConsoleProcessList`, and then open `CONIN$`/`CONOUT$`. They do not use `CREATE_NO_WINDOW`. Real API checks cover injected records, readback, restoration failures, constructor failures, normal drop and panic unwinding. The test fixture is also unsafe: its borrowed standard handles are held open by files, all record unions are initialized by their discriminators, and console-global changes are restricted to dedicated test processes. See [Windows validation](WINDOWS_VALIDATION.md) for host results and limits.

## Remaining risk

Signal ownership is process global. Code outside terra-term that replaces SIGWINCH while a `Terminal` exists may have its handler overwritten during restoration. On Windows, unrelated code or another process attached to the same console can still change console-global modes/code pages or consume its input. The lease coordinates this crate's terminal instances only. Forced process termination, aborting panics, or detaching the console cannot guarantee restoration.

## Foreground child API (2026-10-06)

Unix `Backend::run_command` duplicates owned controlling-terminal descriptors,
saves SIGTTOU disposition and foreground group, and creates a private child group.
Its pre_exec callback uses only async-signal-safe libc functions to claim the
terminal and restore the child's signal disposition before exec. The parent
restores foreground ownership and signal disposition on both spawn/wait failure
and success. Terminal::run_command attempts resume even after suspend/child error.
The single terminal lease owns this process-wide lifecycle.

Windows adds/removes a fixed function console handler around child wait. It only
handles Ctrl+C/Break in the parent; children do not inherit function handlers.
This remains native Windows runtime acceptance work despite GNU compilation.
Existing PTY failure restoration plus TDX's foreground-child PTY checks provide
Unix evidence. There is no terminal mode/signal duplication in TDX.
