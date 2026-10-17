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
    let (terminal, _master, _slave, _original) = terminal()?;
    let received = Rc::new(RefCell::new(false));
    let sender_slot = Rc::new(RefCell::new(None));
    Application::new(
        Background {
            received: received.clone(),
            sender_slot: sender_slot.clone(),
            worker: None,
        },
        Desktop::default(),
    )
    .run_on(terminal)?;
    assert!(
        *received.borrow(),
        "worker must wake a loop blocked without input"
    );
    let sender = sender_slot.borrow();
    let sender = sender.as_ref().unwrap();
    assert!(sender.is_closed());
    assert!(sender.send(9).is_err());
    // Notification bursts must never block a producer or starve ready keyboard input.
    let (mut terminal, master, _slave, _original) = self::terminal()?;
    let waker = terminal.waker();
    for _ in 0..1000 {
        waker.wake()?;
    }
    assert_eq!(
        unsafe { libc::write(master.as_raw_fd(), b"x".as_ptr().cast(), 1) },
        1
    );
    assert!(
        matches!(terminal.try_event()?, Some(terratui::terrabox::Event::Key(k)) if k.key == terratui::terrabox::Key::Char('x'))
    );
    let started = Instant::now();
    assert!(
        terminal
            .wait_event(Some(Duration::from_millis(200)))?
            .is_none()
    );
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "pending notification must wake immediately"
    );
    drop(terminal);
    // Handles own their resources; keeping one after terminal teardown is safe.
    waker.wake()?;

    Ok(())
}

struct Background {
    received: Rc<RefCell<bool>>,
    sender_slot: Rc<RefCell<Option<terratui::EventSender<u8>>>>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl App for Background {
    type Message = u8;
    fn init(&mut self, context: &mut Context<'_, u8>) -> io::Result<()> {
        let sender = context.sender();
        *self.sender_slot.borrow_mut() = Some(sender.clone());
        // Safety deadline prevents a broken wakeup from hanging the suite.
        context.set_timer(
            TimerId(99),
            Deadline::after(Duration::from_secs(2)).unwrap(),
        );
        self.worker = Some(std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(25));
            sender.send(7).unwrap();
        }));
        Ok(())
    }
    fn event(&mut self, event: AppEvent<u8>, context: &mut Context<'_, u8>) -> io::Result<()> {
        match event {
            AppEvent::User(7) => {
                *self.received.borrow_mut() = true;
                context.quit();
            }
            AppEvent::Timer(_) => return Err(io::Error::other("worker wakeup timed out")),
            _ => {}
        }
        Ok(())
    }
    fn shutdown(&mut self, context: &mut Context<'_, u8>) -> io::Result<()> {
        assert!(context.sender().is_closed());
        self.worker.take().unwrap().join().unwrap();
        Ok(())
    }
}
