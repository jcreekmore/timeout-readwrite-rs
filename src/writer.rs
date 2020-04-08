// Copyright 2017 Jonathan Creekmore
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(unix)]
use nix::poll::PollFlags;
use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
use std::os::raw::c_int;
use std::time::Duration;
#[cfg(windows)]
use winapi::um;

use super::utils;

/// The `TimeoutWriter` struct adds write timeouts to any writer.
///
/// The `write` call on a `Write` instance can block forever until data cannot be written.
/// A `TimeoutWriter` will wait until data can be written, up until an optional timeout,
/// before actually performing the `write` operation on the underlying writer.
///
/// If any `Write` operation times out, the method called will return
/// an `io::ErrorKind::TimedOut` variant as the value of `io::Error`. All other
/// error values that would normally be produced by the underlying implementation
/// of the `Write` trait could also be produced by the `TimeoutWriter`.
pub struct TimeoutWriter<H> {
    timeout: Option<c_int>,
    handle: H,
}

#[cfg(unix)]
impl<H> Write for TimeoutWriter<H>
where
    H: Write + AsRawFd,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        utils::wait_until_ready(self.timeout, &self.handle, PollFlags::POLLOUT)?;
        self.handle.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        utils::wait_until_ready(self.timeout, &self.handle, PollFlags::POLLOUT)?;
        self.handle.flush()
    }
}

#[cfg(windows)]
impl<H> Write for TimeoutWriter<H>
where
    H: Write + AsRawHandle,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if let Some(timeout) = self.timeout {
            let handle = self.handle.as_raw_handle();
            let mut timeouts = unsafe { ::std::mem::zeroed::<um::winbase::COMMTIMEOUTS>() };
            timeouts.WriteTotalTimeoutConstant = timeout as u32;

            unsafe { um::commapi::SetCommTimeouts(handle, &mut timeouts) };
        }
        self.handle.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        if let Some(timeout) = self.timeout {
            let handle = self.handle.as_raw_handle();
            let mut timeouts = unsafe { ::std::mem::zeroed::<um::winbase::COMMTIMEOUTS>() };
            timeouts.WriteTotalTimeoutConstant = timeout as u32;

            unsafe { um::commapi::SetCommTimeouts(handle, &mut timeouts) };
        }
        self.handle.flush()
    }
}

impl<H> Seek for TimeoutWriter<H>
where
    H: Seek,
{
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.handle.seek(pos)
    }
}

#[cfg(unix)]
impl<H> AsRawFd for TimeoutWriter<H>
where
    H: Write + AsRawFd,
{
    fn as_raw_fd(&self) -> c_int {
        self.handle.as_raw_fd()
    }
}

impl<H> Clone for TimeoutWriter<H>
where
    H: Clone,
{
    fn clone(&self) -> TimeoutWriter<H> {
        TimeoutWriter {
            handle: self.handle.clone(),
            ..*self
        }
    }
}

impl<H> TimeoutWriter<H>
where
    H: Write,
{
    /// Create a new `TimeoutWriter` with an optional timeout.
    ///
    /// # Examples
    ///
    /// This first example creates the `TimeoutWriter` with a 5-second timeout.
    ///
    /// ```
    /// use timeout_readwrite::TimeoutWriter;
    /// use std::fs::File;
    /// use std::time::Duration;
    ///
    /// # fn foo() -> std::io::Result<()> {
    /// let mut f = File::open("file.txt")?;
    /// let mut wtr = TimeoutWriter::new(f, Duration::new(5, 0));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// This example creates the `TimeoutWriter` without a timeout at all.
    ///
    /// ```
    /// use timeout_readwrite::TimeoutWriter;
    /// use std::fs::File;
    /// use std::time::Duration;
    ///
    /// # fn foo() -> std::io::Result<()> {
    /// let mut f = File::open("file.txt")?;
    /// let mut wtr = TimeoutWriter::new(f, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new<T: Into<Option<Duration>>>(handle: H, timeout: T) -> TimeoutWriter<H> {
        TimeoutWriter {
            timeout: timeout.into().map(utils::duration_to_ms),
            handle: handle,
        }
    }
}

pub trait TimeoutWriteExt<H> {
    fn with_timeout<T: Into<Option<Duration>>>(self, timeout: T) -> TimeoutWriter<H>;
}

impl<H> TimeoutWriteExt<H> for H
where
    H: Write,
{
    fn with_timeout<T: Into<Option<Duration>>>(self, timeout: T) -> TimeoutWriter<H> {
        TimeoutWriter::new(self, timeout)
    }
}
