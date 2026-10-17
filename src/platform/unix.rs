use super::Wake;
use crate::Size;
use crate::terminfo::Capabilities;
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd, RawFd};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::time::Duration;

#[cfg(any(target_os = "solaris", target_os = "illumos"))]
use libc::___errno as errno_location;
#[cfg(any(
    target_os = "android",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "cygwin",
    target_os = "nuttx"
))]
use libc::__errno as errno_location;
#[cfg(any(
    target_os = "linux",
    target_os = "l4re",
    target_os = "hurd",
    target_os = "redox",
    target_os = "dragonfly",
    target_os = "emscripten"
))]
use libc::__errno_location as errno_location;
#[cfg(any(target_vendor = "apple", target_os = "freebsd"))]
use libc::__error as errno_location;
#[cfg(target_os = "aix")]
use libc::_Errno as errno_location;
#[cfg(target_os = "haiku")]
use libc::_errnop as errno_location;

static SIGNAL_FD: AtomicI32 = AtomicI32::new(-1);
static SIGNAL_OWNER: AtomicBool = AtomicBool::new(false);
static SIGNAL_READERS: AtomicUsize = AtomicUsize::new(0);
struct SignalLease;
impl SignalLease {
    fn acquire() -> io::Result<Self> {
        SIGNAL_OWNER
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| io::Error::other("a terminal already owns SIGWINCH in this process"))?;
        Ok(Self)
    }
}
impl Drop for SignalLease {
    fn drop(&mut self) {
        SIGNAL_OWNER.store(false, Ordering::Release);
    }
}
extern "C" fn on_resize(_: libc::c_int) {
    // SAFETY: libc exposes this thread's writable errno slot. A signal handler
    // must preserve it even when a full nonblocking pipe makes write fail.
    let errno = unsafe { errno_location() };
    let saved_errno = unsafe { *errno };
    // Pair with teardown so a handler already running on another thread cannot
    // write to a closed (and possibly reused) descriptor.
    SIGNAL_READERS.fetch_add(1, Ordering::SeqCst);
    let fd = SIGNAL_FD.load(Ordering::SeqCst);
    if fd >= 0 {
        let byte: u8 = 1; /* SAFETY: signal-safe write to our pipe. */
        unsafe {
            libc::write(fd, (&byte as *const u8).cast(), 1);
        }
    }
    SIGNAL_READERS.fetch_sub(1, Ordering::SeqCst);
    // SAFETY: restore the same thread-local slot before returning from the signal.
    unsafe { *errno = saved_errno };
}

pub(crate) struct Backend {
    input: RawFd,
    output: RawFd,
    original: libc::termios,
    active: bool,
    needs_restore: bool,
    resize_read: RawFd,
    resize_write: RawFd,
    previous_signal: libc::sigaction,
    caps: Capabilities,
    _owned_fds: Option<(OwnedFd, OwnedFd)>,
    _lease: SignalLease,
}

impl Backend {
    pub fn event_fds(&self) -> (BorrowedFd<'_>, BorrowedFd<'_>) {
        // SAFETY: the backend owns both descriptors for at least this borrow.
        unsafe {
            (
                BorrowedFd::borrow_raw(self.input),
                BorrowedFd::borrow_raw(self.resize_read),
            )
        }
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
    pub fn new(caps: Capabilities) -> io::Result<Self> {
        let tty = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")?;
        let input = OwnedFd::from(tty.try_clone()?);
        let output = OwnedFd::from(tty);
        Self::from_fds(input, output, caps)
    }
    pub fn from_stdio(caps: Capabilities) -> io::Result<Self> {
        Self::new_impl(libc::STDIN_FILENO, libc::STDOUT_FILENO, caps, None)
    }
    pub fn from_fds(input: OwnedFd, output: OwnedFd, caps: Capabilities) -> io::Result<Self> {
        let input_fd = input.as_raw_fd();
        let output_fd = output.as_raw_fd();
        Self::new_impl(input_fd, output_fd, caps, Some((input, output)))
    }
    fn new_impl(
        input: RawFd,
        output: RawFd,
        caps: Capabilities,
        owned_fds: Option<(OwnedFd, OwnedFd)>,
    ) -> io::Result<Self> {
        // SAFETY: isatty examines valid process file descriptors.
        if unsafe { libc::isatty(input) } != 1 || unsafe { libc::isatty(output) } != 1 {
            return Err(io::Error::other("input and output must be terminals"));
        }
        let lease = SignalLease::acquire()?;
        Self::create(input, output, caps, owned_fds, lease)
    }
    fn create(
        input: RawFd,
        output: RawFd,
        caps: Capabilities,
        owned_fds: Option<(OwnedFd, OwnedFd)>,
        lease: SignalLease,
    ) -> io::Result<Self> {
        let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
        // SAFETY: tcgetattr initializes original for a valid terminal descriptor.
        if unsafe { libc::tcgetattr(input, original.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: tcgetattr succeeded.
        let original = unsafe { original.assume_init() };
        let mut pipe = [-1; 2];
        // SAFETY: pipe points to two writable descriptors.
        if unsafe { libc::pipe(pipe.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        for fd in pipe {
            // SAFETY: both descriptors were created by pipe and remain open.
            if unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) } < 0
                || unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0
            {
                let error = io::Error::last_os_error();
                // SAFETY: both pipe descriptors are still owned here.
                unsafe {
                    libc::close(pipe[0]);
                    libc::close(pipe[1]);
                }
                return Err(error);
            }
        }
        // SAFETY: zeroed sigaction is initialized before use.
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        action.sa_sigaction = on_resize as *const () as usize;
        // SAFETY: action's mask is valid memory.
        unsafe {
            libc::sigemptyset(&mut action.sa_mask);
        }
        let mut previous = std::mem::MaybeUninit::<libc::sigaction>::uninit();
        SIGNAL_FD.store(pipe[1], Ordering::SeqCst);
        // SAFETY: sigaction installs our handler and initializes previous.
        if unsafe { libc::sigaction(libc::SIGWINCH, &action, previous.as_mut_ptr()) } != 0 {
            let error = io::Error::last_os_error();
            SIGNAL_FD.store(-1, Ordering::SeqCst);
            // SAFETY: pipe descriptors are open.
            unsafe {
                libc::close(pipe[0]);
                libc::close(pipe[1]);
            }
            return Err(error);
        }
        // SAFETY: sigaction succeeded.
        let previous_signal = unsafe { previous.assume_init() };
        let mut backend = Self {
            input,
            output,
            original,
            active: false,
            needs_restore: false,
            resize_read: pipe[0],
            resize_write: pipe[1],
            previous_signal,
            caps,
            _owned_fds: owned_fds,
            _lease: lease,
        };
        backend.resume()?;
        Ok(backend)
    }
    pub fn size(&self) -> io::Result<Size> {
        let mut ws = std::mem::MaybeUninit::<libc::winsize>::zeroed();
        // SAFETY: ioctl fills winsize for a valid terminal descriptor.
        if unsafe { libc::ioctl(self.output, libc::TIOCGWINSZ, ws.as_mut_ptr()) } < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: zeroed winsize is initialized, and ioctl succeeded.
        let ws = unsafe { ws.assume_init() };
        Ok(Size {
            width: ws.ws_col,
            height: ws.ws_row,
        })
    }
    pub fn wait(&mut self, timeout: Option<Duration>) -> io::Result<Wake> {
        let ms = timeout
            .map(|d| d.as_millis().min(i32::MAX as u128) as i32)
            .unwrap_or(-1);
        let mut fds = [
            libc::pollfd {
                fd: self.input,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: self.resize_read,
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        // SAFETY: fds points to two initialized pollfd values.
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), 2, ms) };
        if ready < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                return Ok(Wake::Resize);
            }
            return Err(e);
        }
        if ready == 0 {
            return Ok(Wake::Timeout);
        }
        if fds[1].revents & libc::POLLIN != 0 {
            let mut bytes = [0u8; 64];
            // SAFETY: pipe is nonblocking and bytes is writable.
            unsafe {
                libc::read(self.resize_read, bytes.as_mut_ptr().cast(), bytes.len());
            }
            return Ok(Wake::Resize);
        }
        if fds[0].revents & libc::POLLIN != 0 {
            return Ok(Wake::Input);
        }
        Err(io::Error::other("terminal input hangup or poll error"))
    }
    pub fn read(&self, bytes: &mut [u8]) -> io::Result<usize> {
        loop {
            // SAFETY: bytes is writable for its length and input is a valid descriptor.
            let n = unsafe { libc::read(self.input, bytes.as_mut_ptr().cast(), bytes.len()) };
            if n >= 0 {
                return Ok(n as usize);
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }
    pub fn write(&self, bytes: &[u8]) -> io::Result<()> {
        let mut rest = bytes;
        while !rest.is_empty() {
            // SAFETY: output is a valid descriptor and rest points to readable bytes.
            let n = unsafe { libc::write(self.output, rest.as_ptr().cast(), rest.len()) };
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "terminal write returned zero",
                ));
            }
            rest = &rest[n as usize..];
        }
        Ok(())
    }
    pub fn run_command(
        &mut self,
        command: &mut std::process::Command,
    ) -> io::Result<std::process::ExitStatus> {
        use std::os::unix::process::CommandExt;
        // Attach all streams to the controlling terminal, even if the application's
        // original stdin/stdout was redirected. Duplicates are child-owned.
        let (input, _) = self.event_fds();
        command.stdin(std::process::Stdio::from(input.try_clone_to_owned()?));
        // SAFETY: self.output is owned by this backend for the whole call.
        let output = unsafe { BorrowedFd::borrow_raw(self.output) };
        command.stdout(std::process::Stdio::from(output.try_clone_to_owned()?));
        command.stderr(std::process::Stdio::from(output.try_clone_to_owned()?));
        // SAFETY: initialized sigaction is saved before any change and restored.
        let foreground = unsafe { libc::tcgetpgrp(self.input) };
        if foreground < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut previous = std::mem::MaybeUninit::<libc::sigaction>::uninit();
        let mut ignored: libc::sigaction = unsafe { std::mem::zeroed() };
        ignored.sa_sigaction = libc::SIG_IGN;
        unsafe {
            libc::sigemptyset(&mut ignored.sa_mask);
        }
        if unsafe { libc::sigaction(libc::SIGTTOU, &ignored, previous.as_mut_ptr()) } < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful sigaction initialized the previous action.
        let previous = unsafe { previous.assume_init() };
        let fd = self.input;
        command.process_group(0);
        // SAFETY: child callback only calls async-signal-safe libc functions.
        // The inherited ignored SIGTTOU permits claiming the terminal before exec.
        unsafe {
            command.pre_exec(move || {
                if libc::tcsetpgrp(fd, libc::getpgrp()) < 0 {
                    return Err(io::Error::last_os_error());
                }
                if libc::sigaction(libc::SIGTTOU, &previous, std::ptr::null_mut()) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let result = command.status();
        // SAFETY: foreground belongs to the caller; SIGTTOU remains ignored here.
        let restored = unsafe { libc::tcsetpgrp(self.input, foreground) };
        let restore_error = if restored < 0 {
            Some(io::Error::last_os_error())
        } else {
            None
        };
        let signal_restored =
            unsafe { libc::sigaction(libc::SIGTTOU, &previous, std::ptr::null_mut()) };
        if let Some(error) = restore_error {
            return Err(error);
        }
        if signal_restored < 0 {
            return Err(io::Error::last_os_error());
        }
        result
    }
    pub fn suspend(&mut self) -> io::Result<()> {
        if !self.needs_restore {
            return Ok(());
        }
        self.active = false;
        let mut seq = b"\x1b[?2004l\x1b[?1006l\x1b[?1015l\x1b[?1002l\x1b[?1000l".to_vec();
        seq.extend_from_slice(&self.caps.reset);
        seq.extend_from_slice(b"\x1b[?7h");
        seq.extend_from_slice(&self.caps.show_cursor);
        seq.extend_from_slice(&self.caps.exit_keypad);
        seq.extend_from_slice(&self.caps.exit_screen);
        let output = self.write(&seq);
        let raw = set_termios(self.input, &self.original);
        if output.is_ok() && raw.is_ok() {
            self.needs_restore = false;
        }
        output?;
        raw
    }
    pub fn resume(&mut self) -> io::Result<()> {
        if self.active {
            return Ok(());
        }
        if self.needs_restore {
            self.suspend()?;
        }
        let mut raw = self.original;
        // SAFETY: raw points to a valid termios structure.
        unsafe {
            libc::cfmakeraw(&mut raw);
        }
        set_termios(self.input, &raw)?;
        self.active = true;
        self.needs_restore = true;
        let mut seq = self.caps.enter_screen.clone();
        seq.extend_from_slice(&self.caps.enter_keypad);
        seq.extend_from_slice(b"\x1b[?7l");
        seq.extend_from_slice(&self.caps.hide_cursor);
        seq.extend_from_slice(b"\x1b[?2004h");
        seq.extend_from_slice(&self.caps.reset);
        if let Err(e) = self.write(&seq) {
            let _ = self.suspend();
            return Err(e);
        }
        Ok(())
    }
    pub fn set_mouse(&self, enabled: bool) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        if enabled {
            self.write(b"\x1b[?1000h\x1b[?1002h\x1b[?1015h\x1b[?1006h")
        } else {
            self.write(b"\x1b[?1006l\x1b[?1015l\x1b[?1002l\x1b[?1000l")
        }
    }
    pub fn clear_screen(&self) -> io::Result<()> {
        self.write(&self.caps.clear)
    }
}
impl Drop for Backend {
    fn drop(&mut self) {
        let _ = self.suspend();
        SIGNAL_FD.store(-1, Ordering::SeqCst);
        // SAFETY: previous_signal was saved when installing our handler.
        unsafe {
            libc::sigaction(libc::SIGWINCH, &self.previous_signal, std::ptr::null_mut());
        }
        while SIGNAL_READERS.load(Ordering::SeqCst) != 0 {
            std::thread::yield_now();
        }
        // SAFETY: no in-flight handler can still access these owned descriptors.
        unsafe {
            libc::close(self.resize_read);
            libc::close(self.resize_write);
        }
    }
}

fn set_termios(fd: RawFd, value: &libc::termios) -> io::Result<()> {
    loop {
        // SAFETY: fd is a terminal descriptor and value is initialized.
        if unsafe { libc::tcsetattr(fd, libc::TCSAFLUSH, value) } == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::fd::FromRawFd;

    #[test]
    fn resize_handler_preserves_errno_and_read_retries_interruption() -> io::Result<()> {
        let mut master = -1;
        let mut slave = -1;
        // SAFETY: initialized output slots receive fresh PTY descriptors.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                )
            },
            0
        );
        // SAFETY: openpty transferred ownership of both descriptors.
        let master = unsafe { OwnedFd::from_raw_fd(master) };
        let slave = unsafe { OwnedFd::from_raw_fd(slave) };
        let backend = Backend::from_fds(slave.try_clone()?, slave, Capabilities::builtin("xterm"))?;
        // Fill the nonblocking resize pipe so the handler's write sets EAGAIN.
        let bytes = [1u8; 4096];
        // SAFETY: the backend owns the write descriptor and bytes is readable.
        while unsafe { libc::write(backend.resize_write, bytes.as_ptr().cast(), bytes.len()) } > 0 {
        }
        assert_eq!(io::Error::last_os_error().kind(), io::ErrorKind::WouldBlock);
        // SAFETY: errno_location points to the current thread's errno slot.
        unsafe { *errno_location() = libc::EDOM };
        on_resize(libc::SIGWINCH);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::EDOM));
        // SAFETY: pthread_self returns this live thread's ID.
        let reader = unsafe { libc::pthread_self() };
        let worker = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            // SAFETY: reader stays live until this thread has joined. The backend
            // owns the installed handler throughout the blocking read.
            assert_eq!(unsafe { libc::pthread_kill(reader, libc::SIGWINCH) }, 0);
            std::thread::sleep(Duration::from_millis(20));
            // SAFETY: master remains open and the byte is readable.
            assert_eq!(
                unsafe { libc::write(master.as_raw_fd(), b"q".as_ptr().cast(), 1) },
                1
            );
            master
        });
        let mut input = [0; 1];
        assert_eq!(backend.read(&mut input)?, 1);
        assert_eq!(input, *b"q");
        let _master = worker.join().unwrap();
        Ok(())
    }
}
