//! Silence process stdout while invoking CLI helpers that println.
//! MCP stdio must stay clean — only JSON-RPC lines on stdout.

use std::fs::OpenOptions;
use std::os::fd::{IntoRawFd, RawFd};

pub struct StdoutSilence {
    saved: RawFd,
}

impl StdoutSilence {
    pub fn enter() -> std::io::Result<Self> {
        let null = OpenOptions::new().write(true).open("/dev/null")?;
        let null_fd = null.into_raw_fd();
        let saved = unsafe { libc::dup(1) };
        if saved < 0 {
            unsafe { libc::close(null_fd) };
            return Err(std::io::Error::last_os_error());
        }
        if unsafe { libc::dup2(null_fd, 1) } < 0 {
            let err = std::io::Error::last_os_error();
            unsafe {
                libc::close(null_fd);
                libc::close(saved);
            }
            return Err(err);
        }
        unsafe { libc::close(null_fd) };
        Ok(Self { saved })
    }
}

impl Drop for StdoutSilence {
    fn drop(&mut self) {
        unsafe {
            libc::dup2(self.saved, 1);
            libc::close(self.saved);
        }
    }
}

pub fn quiet<T>(f: impl FnOnce() -> T) -> T {
    let _guard = StdoutSilence::enter().ok();
    f()
}
