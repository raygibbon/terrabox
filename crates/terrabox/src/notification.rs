//! Coalescing wakeups independent of terminal input and resize notifications.
use std::{io, sync::Arc};

/// Thread-safe notification handle. Wakeups coalesce; store messages separately.
#[derive(Clone)]
pub struct Waker(Arc<Notification>);
impl Waker {
    pub fn wake(&self) -> io::Result<()> {
        self.0.signal()
    }
}

#[cfg(unix)]
pub(crate) struct Notification {
    reader: std::os::unix::net::UnixStream,
    writer: std::os::unix::net::UnixStream,
}
#[cfg(unix)]
impl Notification {
    pub fn new() -> io::Result<Self> {
        let (reader, writer) = std::os::unix::net::UnixStream::pair()?;
        reader.set_nonblocking(true)?;
        writer.set_nonblocking(true)?;
        Ok(Self { reader, writer })
    }
    pub fn raw(&self) -> std::os::fd::RawFd {
        use std::os::fd::AsRawFd;
        self.reader.as_raw_fd()
    }
    fn signal(&self) -> io::Result<()> {
        use std::io::Write;
        loop {
            match (&self.writer).write(&[1]) {
                Ok(_) => return Ok(()),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
    }
    pub fn clear(&self) {
        use std::io::Read;
        let mut bytes = [0; 256];
        // Bound each drain so sustained producers cannot hold the UI here.
        // Remaining bytes keep the descriptor readable on the next wait.
        loop {
            match (&self.reader).read(&mut bytes) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                _ => break,
            }
        }
    }
}
#[cfg(windows)]
pub(crate) struct Notification(std::os::windows::io::OwnedHandle);
#[cfg(windows)]
impl Notification {
    pub fn new() -> io::Result<Self> {
        use std::os::windows::io::FromRawHandle;
        // SAFETY: no name or security descriptor; returned handle is owned.
        let handle = unsafe {
            windows_sys::Win32::System::Threading::CreateEventW(
                std::ptr::null(),
                0,
                0,
                std::ptr::null(),
            )
        };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(unsafe {
            std::os::windows::io::OwnedHandle::from_raw_handle(handle)
        }))
    }
    pub fn raw(&self) -> windows_sys::Win32::Foundation::HANDLE {
        use std::os::windows::io::AsRawHandle;
        self.0.as_raw_handle()
    }
    fn signal(&self) -> io::Result<()> {
        // SAFETY: owned event handle remains valid during the call.
        if unsafe { windows_sys::Win32::System::Threading::SetEvent(self.raw()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}
impl Notification {
    pub fn waker(self: &Arc<Self>) -> Waker {
        Waker(self.clone())
    }
}
