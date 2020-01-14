// Copyright 2017 Jonathan Creekmore
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use nix::libc::c_int;
use nix::poll::PollFlags;
use std::io::Read;
use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom;
use std::os::unix::io::AsRawFd;
use std::time::Duration;

use super::utils;

/// The `TimeoutReaderMut` struct adds read timeouts to any reader.
///
/// The `read` call on a `Read` instance will block forever until data is available.
/// A `TimeoutReaderMut` will wait until data is available, up until an optional timeout,
/// before actually performing the `read` operation.
///
/// If any `Read` operation times out, the method called will return
/// an `io::ErrorKind::TimedOut` variant as the value of `io::Error`. All other
/// error values that would normally be produced by the underlying implementation
/// of the `Read` trait could also be produced by the `TimeoutReaderMut`.
pub struct TimeoutReaderMut<'a, H>
where
    H: Read + AsRawFd,
{
    timeout: Option<c_int>,
    handle: &'a mut H,
}

impl<H> Read for TimeoutReaderMut<'_, H>
where
    H: Read + AsRawFd,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        utils::wait_until_ready(self.timeout, &*self.handle, PollFlags::POLLIN)?;
        self.handle.read(buf)
    }
}

impl<H> Seek for TimeoutReaderMut<'_, H>
where
    H: Read + AsRawFd + Seek,
{
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.handle.seek(pos)
    }
}

impl<H> AsRawFd for TimeoutReaderMut<'_, H>
where
    H: Read + AsRawFd,
{
    fn as_raw_fd(&self) -> c_int {
        self.handle.as_raw_fd()
    }
}

impl<'a, H> TimeoutReaderMut<'a, H>
where
    H: Read + AsRawFd,
{
    /// Create a new `TimeoutReaderMut` with an optional timeout.
    ///
    /// # Examples
    ///
    /// This first example creates the `TimeoutReaderMut` with a 5-second timeout.
    ///
    /// ```
    /// use timeout_readwrite::TimeoutReaderMut;
    /// use std::fs::File;
    /// use std::time::Duration;
    ///
    /// # fn foo() -> std::io::Result<()> {
    /// let mut f = File::open("file.txt")?;
    /// let mut rdr = TimeoutReaderMut::new(f, Duration::new(5, 0));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// This example creates the `TimeoutReaderMut` without a timeout at all.
    ///
    /// ```
    /// use timeout_readwrite::TimeoutReaderMut;
    /// use std::fs::File;
    /// use std::time::Duration;
    ///
    /// # fn foo() -> std::io::Result<()> {
    /// let mut f = File::open("file.txt")?;
    /// let mut rdr = TimeoutReaderMut::new(f, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new<T: Into<Option<Duration>>>(handle: &'a mut H, timeout: T) -> TimeoutReaderMut<H> {
        TimeoutReaderMut {
            timeout: timeout.into().map(utils::duration_to_ms),
            handle: handle,
        }
    }
}

pub trait TimeoutReadMutExt<H>
where
    H: Read + AsRawFd,
{
    fn with_timeout<T: Into<Option<Duration>>>(&mut self, timeout: T) -> TimeoutReaderMut<H>;
}

impl<H> TimeoutReadMutExt<H> for H
where
    H: Read + AsRawFd,
{
    fn with_timeout<T: Into<Option<Duration>>>(&mut self, timeout: T) -> TimeoutReaderMut<H> {
        TimeoutReaderMut::new(self, timeout)
    }
}
