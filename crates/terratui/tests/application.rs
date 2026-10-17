#![cfg(target_os = "linux")]

use std::cell::RefCell;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::rc::Rc;
use std::time::{Duration, Instant};
use terratui::terrabox::{Deadline, Terminal};
use terratui::{App, AppEvent, Application, Context, Desktop, TimerId};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    InitError,
    EventError,
    Panic,
}

struct Model {
    mode: Mode,
    log: Rc<RefCell<Vec<&'static str>>>,
}

impl App for Model {
    type Message = ();

    fn init(&mut self, context: &mut Context<'_, ()>) -> io::Result<()> {
        self.log.borrow_mut().push("init");
        if self.mode == Mode::InitError {
            return Err(io::Error::other("init failure"));
        }
        context.post(());
        context.set_timer(TimerId(1), Deadline(Instant::now()));
        context.set_timer(TimerId(3), Deadline(Instant::now()));
        context.cancel_timer(TimerId(3));
        Ok(())
    }

    fn event(&mut self, event: AppEvent<()>, context: &mut Context<'_, ()>) -> io::Result<()> {
        match event {
            AppEvent::Timer(TimerId(1)) => {
                self.log.borrow_mut().push("timer1");
                context.set_timer(
                    TimerId(2),
                    Deadline::after(Duration::from_millis(5)).unwrap(),
                );
            }
            AppEvent::User(()) => {
                self.log.borrow_mut().push("message");
                if self.mode == Mode::EventError {
                    return Err(io::Error::other("event failure"));
                }
                assert!(self.mode != Mode::Panic, "event panic");
            }
            AppEvent::Timer(TimerId(2)) => {
                self.log.borrow_mut().push("timer2");
                context.quit();
            }
            _ => panic!("unexpected event"),
        }
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Context<'_, ()>) -> io::Result<()> {
        self.log.borrow_mut().push("shutdown");
        Ok(())
    }
}

fn terminal() -> io::Result<(Terminal, OwnedFd, OwnedFd, libc::termios)> {
    let mut master = -1;
    let mut slave = -1;
    let size = libc::winsize {
        ws_row: 6,
        ws_col: 20,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
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
    let master = unsafe { OwnedFd::from_raw_fd(master) };
    let slave = unsafe { OwnedFd::from_raw_fd(slave) };
    let mut original = std::mem::MaybeUninit::uninit();
    if unsafe { libc::tcgetattr(slave.as_raw_fd(), original.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let original = unsafe { original.assume_init() };
    let terminal = Terminal::from_fds(slave.try_clone()?, slave.try_clone()?)?;
    Ok((terminal, master, slave, original))
}

#[test]
fn timers_messages_shutdown_errors_and_unwinding_restore_the_terminal() -> io::Result<()> {
    unsafe {
        std::env::set_var("TERM", "xterm");
    }
    for mode in [Mode::Normal, Mode::InitError, Mode::EventError, Mode::Panic] {
        let (terminal, _master, slave, original) = terminal()?;
        let log = Rc::new(RefCell::new(Vec::new()));
        let application = Application::new(
            Model {
                mode,
                log: log.clone(),
            },
            Desktop::default(),
        );
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            application.run_on(terminal)
        }));
        let mut restored = std::mem::MaybeUninit::uninit();
        assert_eq!(
            unsafe { libc::tcgetattr(slave.as_raw_fd(), restored.as_mut_ptr()) },
            0
        );
        let restored = unsafe { restored.assume_init() };
        assert_eq!(original.c_lflag, restored.c_lflag);
        assert_eq!(original.c_iflag, restored.c_iflag);
        assert_eq!(original.c_oflag, restored.c_oflag);
        assert_eq!(original.c_cc, restored.c_cc);
        match mode {
            Mode::Normal => {
                result.unwrap()?;
                assert_eq!(
                    *log.borrow(),
                    ["init", "timer1", "message", "timer2", "shutdown"]
                );
            }
            Mode::InitError | Mode::EventError => {
                assert!(result.unwrap().is_err());
                assert_eq!(log.borrow().last(), Some(&"shutdown"));
            }
            Mode::Panic => assert!(result.is_err()),
        }
    }
    Ok(())
}
