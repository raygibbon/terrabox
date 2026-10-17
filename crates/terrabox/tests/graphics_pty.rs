//! A protocol responder, not a Kitty emulator or visual rendering verification.
#![cfg(target_os = "linux")]
use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use terrabox::{Capability, Event, Key, Support, Terminal};
fn pty() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut master = -1;
    let mut slave = -1;
    let size = libc::winsize {
        ws_row: 12,
        ws_col: 40,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: writable descriptor pointers and initialized winsize.
    if unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            &size,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openpty transferred ownership of both descriptors.
    Ok(unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) })
}
fn termios(fd: &OwnedFd) -> io::Result<libc::termios> {
    let mut t = std::mem::MaybeUninit::uninit();
    // SAFETY: tcgetattr initializes t on success for this live descriptor.
    if unsafe { libc::tcgetattr(fd.as_raw_fd(), t.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { t.assume_init() })
}
fn send(fd: &OwnedFd, bytes: &[u8]) {
    // SAFETY: bytes remains readable throughout this synchronous small write.
    assert_eq!(
        unsafe { libc::write(fd.as_raw_fd(), bytes.as_ptr().cast(), bytes.len()) },
        bytes.len() as isize
    );
}
fn start_reader(master: OwnedFd, reply: bool) -> (Arc<AtomicBool>, thread::JoinHandle<Vec<u8>>) {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let reader = thread::spawn(move || {
        let mut out = Vec::new();
        let mut replied = false;
        loop {
            let mut poll = libc::pollfd {
                fd: master.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: one writable initialized pollfd.
            let ready = unsafe { libc::poll(&mut poll, 1, 20) };
            if ready > 0 && poll.revents & libc::POLLIN != 0 {
                let mut bytes = [0; 8192];
                // SAFETY: bytes is writable and master remains owned by this thread.
                let count = unsafe {
                    libc::read(master.as_raw_fd(), bytes.as_mut_ptr().cast(), bytes.len())
                };
                if count > 0 {
                    out.extend_from_slice(&bytes[..count as usize]);
                } else {
                    break;
                }
                if reply && !replied {
                    let text = String::from_utf8_lossy(&out);
                    // Wait for the complete query batch, including the optional graphics query.
                    #[cfg(feature = "kitty-graphics")]
                    let id = text
                        .split("a=q,t=d,f=24,s=1,v=1,i=")
                        .nth(1)
                        .and_then(|s| s.split(';').next())
                        .and_then(|s| s.parse::<u32>().ok());
                    #[cfg(not(feature = "kitty-graphics"))]
                    let id = Some(0u32);
                    if text.contains("+q524742")
                        && let Some(id) = id
                    {
                        send(&master,b"a\x1b[A\x1b[200~paste\x1b[201~\x1b[?1006;2$y\x1b[?2004;2$y\x1b[?0u\x1bP1+r436f=323536;524742=38\x1b\\");
                        #[cfg(feature = "kitty-graphics")]
                        send(&master, format!("\x1b_Gi={id};OK\x1b\\").as_bytes());
                        #[cfg(not(feature = "kitty-graphics"))]
                        let _ = id;
                        replied = true;
                    }
                }
            } else if flag.load(Ordering::Acquire) {
                break;
            }
        }
        out
    });
    (stop, reader)
}
#[test]
fn probes_preserve_input_time_out_and_coordinate_graphics_lifecycle() -> io::Result<()> {
    // SAFETY: the only test in this binary changes these environment variables.
    unsafe {
        std::env::set_var("TERM", "xterm-256color");
        std::env::remove_var("TMUX");
        std::env::remove_var("STY");
    }
    let (master, slave) = pty()?;
    let original = termios(&slave)?;
    let (stop, reader) = start_reader(master, true);
    let mut terminal = Terminal::from_fds(slave.try_clone()?, slave.try_clone()?)?;
    terminal.probe_capabilities(Duration::from_millis(100))?;
    assert_eq!(terminal.capabilities().mouse.support, Support::Supported);
    assert_eq!(
        terminal.capabilities().truecolor.support,
        Support::Supported
    );
    assert_eq!(
        terminal.capabilities().colors256.support,
        Support::Supported
    );
    assert!(matches!(terminal.try_event()?,Some(Event::Key(k)) if k.key==Key::Char('a')));
    assert!(matches!(terminal.try_event()?,Some(Event::Key(k)) if k.key==Key::Up));
    assert!(matches!(terminal.try_event()?,Some(Event::Paste(p)) if p.bytes==b"paste"));
    terminal.probe_capabilities(Duration::from_millis(100))?; // Cached; no second batch.
    #[cfg(feature = "kitty-graphics")]
    {
        use terrabox::{Rect, Style, graphics::RgbaImage};
        assert!(terminal.graphics_available());
        assert_eq!(
            terminal.capabilities().kitty_animation.support,
            Support::Unknown
        );
        let data = RgbaImage::new(2, 2, vec![255; 16])?;
        let id = terminal.upload_image(&data)?;
        let placement = terminal.place_image(id, Rect::new(2, 3, 4, 2))?;
        terminal.put_str(0, 0, "cell-output", Style::default());
        terminal.set_cursor(8, 6);
        terminal.present()?;
        terminal.move_image(placement, Rect::new(3, 4, 4, 2))?;
        terminal.present()?;
        let new_size = libc::winsize {
            ws_row: 14,
            ws_col: 42,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: ioctl and raise operate on the live test PTY/process.
        assert_eq!(
            unsafe { libc::ioctl(slave.as_raw_fd(), libc::TIOCSWINSZ, &new_size) },
            0
        );
        assert_eq!(unsafe { libc::raise(libc::SIGWINCH) }, 0);
        assert!(matches!(
            terminal.read_event_timeout(Duration::from_secs(1))?,
            Some(Event::Resize(_))
        ));
        assert!(
            terminal
                .move_image(placement, Rect::new(0, 0, 1, 1))
                .is_err()
        );
        terminal.place_image(id, Rect::new(1, 1, 2, 2))?;
        terminal.present()?; // pixels reused after resize.
        terminal.suspend()?;
        assert_eq!(original.c_lflag, termios(&slave)?.c_lflag);
        terminal.resume()?;
        assert!(terminal.place_image(id, Rect::new(0, 0, 1, 1)).is_err()); // Suspend invalidates handles.
        terminal.upload_image(&data)?; // Drop must clean this final image before screen restoration.
    }
    terminal.override_capability(Capability::KittyAnimation, Support::Unsupported);
    assert_eq!(
        terminal.capabilities().kitty_animation.support,
        Support::Unsupported
    );
    drop(terminal);
    assert_eq!(original.c_lflag, termios(&slave)?.c_lflag);
    assert_eq!(original.c_iflag, termios(&slave)?.c_iflag);
    stop.store(true, Ordering::Release);
    let output = reader.join().unwrap();
    let text = String::from_utf8_lossy(&output);
    assert_eq!(text.matches("?1006$p").count(), 1);
    #[cfg(feature = "kitty-graphics")]
    {
        assert_eq!(text.matches("a=t,t=d,f=32").count(), 2);
        assert!(text.find("cell-output").unwrap() < text.find("a=p,").unwrap());
        assert!(text.rfind("d=I,").unwrap() < text.rfind("?1049l").unwrap());
        assert!(text.contains("C=1,z=-1"));
    }
    // Silence stays unknown, and the timeout doesn't require terminal input.
    let (master, slave) = pty()?;
    let (stop, reader) = start_reader(master, false);
    let mut terminal = Terminal::from_fds(slave.try_clone()?, slave.try_clone()?)?;
    let started = Instant::now();
    terminal.probe_capabilities(Duration::from_millis(20))?;
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(
        terminal.capabilities().kitty_graphics.support,
        Support::Unknown
    );
    assert!(terminal.probe_capabilities(Duration::from_secs(3)).is_err());
    #[cfg(feature = "kitty-graphics")]
    assert!(
        terminal
            .upload_image(&terrabox::graphics::RgbaImage::new(1, 1, vec![0; 4])?)
            .is_err()
    );
    drop(terminal);
    stop.store(true, Ordering::Release);
    reader.join().unwrap();
    // Multiplexers remain unknown and emit no detection/graphics commands.
    unsafe {
        std::env::set_var("TMUX", "test-multiplexer");
    }
    let (master, slave) = pty()?;
    let (stop, reader) = start_reader(master, false);
    let mut terminal = Terminal::from_fds(slave.try_clone()?, slave.try_clone()?)?;
    terminal.probe_capabilities(Duration::from_millis(20))?;
    assert!(terminal.capabilities().multiplexer);
    terminal.override_capability(Capability::KittyGraphics, Support::Supported);
    #[cfg(feature = "kitty-graphics")]
    assert!(!terminal.graphics_available());
    drop(terminal);
    stop.store(true, Ordering::Release);
    let output = reader.join().unwrap();
    assert!(!output.windows(3).any(|b| b == b"\x1b_G"));
    assert!(!output.windows(7).any(|b| b == b"?1006$p"));
    Ok(())
}
