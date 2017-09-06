// Copyright 2017 Jonathan Creekmore
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg(unix)]

//! Provides TimeoutReader and TimeoutWriter structs to time out reads and
//! writes, respectively.
//!
//! # Example: read from a process with a 5-second timeout
//!
//! Given a process that writes to standard out, read from the output once it is there,
//! but fail if you have to wait longer than 5-seconds for data to be present on standard
//! out.
//!
//! ```rust
//! use std::io::{Read, Result};
//! use std::process;
//! use std::time::Duration;
//! use timeout_readwrite::TimeoutReader;
//!
//! fn read_command(mut cmd: process::Command) -> Result<usize> {
//!     let child = cmd.stdout(process::Stdio::piped())
//!        .stderr(process::Stdio::null())
//!        .spawn()
//!        .expect("spawing did not succeed");
//!     let stdout = child.stdout.expect("stdout must be there");
//!
//!     let mut data = String::new();
//!     let mut rdr = TimeoutReader::new(stdout, Duration::new(5, 0));
//!     rdr.read_to_string(&mut data)
//! }
//! ```

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

extern crate nix;

use nix::libc::c_int;
use nix::poll;
use std::cmp;
use std::io::{Error, ErrorKind, Result};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::slice;
use std::time::Duration;

/// Convert from a duration into milliseconds as the c_int type that poll expects.
/// If the duration exceeds the number of milliseconds that can fit into a c_int,
/// saturate the time to the max_value of c_int.
fn duration_to_ms(duration: Duration) -> c_int {
    let secs = cmp::min(duration.as_secs(), c_int::max_value() as u64) as c_int;
    let nanos = duration.subsec_nanos() as c_int;

    secs.saturating_mul(1_000).saturating_add(nanos / 1_000_000)
}

/// Wait until `to_fd` receives the poll event from `events`, up to `timeout` length
/// of time.
fn wait_until_ready<R: AsRawFd>(timeout: &Option<c_int>,
                                to_fd: &R,
                                events: poll::EventFlags)
                                -> Result<()> {
    if let Some(timeout) = *timeout {
        let mut pfd = poll::PollFd::new(to_fd.as_raw_fd(), events);
        let mut s = unsafe { slice::from_raw_parts_mut(&mut pfd, 1) };

        let retval = poll::poll(&mut s, timeout).map_err(|e| Error::new(ErrorKind::Other, e))?;
        if retval == 0 {
            return Err(Error::new(ErrorKind::TimedOut, "timed out waiting for fd to be ready"));
        }
    }
    Ok(())
}

/// The `TimeoutReader` struct adds read timeouts to any reader.
///
/// The `read` call on a `Read` instance will block forever until data is available.
/// A `TimeoutReader` will wait until data is available, up until an optional timeout,
/// before actually performing the `read` operation.
pub struct TimeoutReader<H>
    where H: Read + AsRawFd
{
    timeout: Option<c_int>,
    handle: H,
}

impl<H> Read for TimeoutReader<H>
    where H: Read + AsRawFd
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        wait_until_ready(&self.timeout, &self.handle, poll::POLLIN)?;
        self.handle.read(buf)
    }
}

impl<H> Clone for TimeoutReader<H>
    where H: Read + AsRawFd + Clone
{
    fn clone(&self) -> TimeoutReader<H> {
        TimeoutReader { handle: self.handle.clone(), ..*self }
    }
}

impl<H> TimeoutReader<H>
    where H: Read + AsRawFd
{
    /// Create a new `TimeoutReader` with an optional timeout.
    ///
    /// # Examples
    ///
    /// This first example creates the `TimeoutReader` with a 5-second timeout.
    ///
    /// ```
    /// use timeout_readwrite::TimeoutReader;
    /// use std::fs::File;
    /// use std::time::Duration;
    ///
    /// # fn foo() -> std::io::Result<()> {
    /// let mut f = File::open("file.txt")?;
    /// let mut rdr = TimeoutReader::new(f, Duration::new(5, 0));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// This example creates the `TimeoutReader` without a timeout at all.
    ///
    /// ```
    /// use timeout_readwrite::TimeoutReader;
    /// use std::fs::File;
    /// use std::time::Duration;
    ///
    /// # fn foo() -> std::io::Result<()> {
    /// let mut f = File::open("file.txt")?;
    /// let mut rdr = TimeoutReader::new(f, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new<T: Into<Option<Duration>>>(handle: H, timeout: T) -> TimeoutReader<H> {
        TimeoutReader {
            timeout: timeout.into().map(duration_to_ms),
            handle: handle,
        }
    }
}

/// The `TimeoutWriter` struct adds write timeouts to any writer.
///
/// The `write` call on a `Write` instance can block forever until data cannot be written.
/// A `TimeoutWriter` will wait until data can be written, up until an optional timeout,
/// before actually performing the `write` operation on the underlying writer.
pub struct TimeoutWriter<H>
    where H: Write + AsRawFd
{
    timeout: Option<c_int>,
    handle: H,
}

impl<H> Write for TimeoutWriter<H>
    where H: Write + AsRawFd
{
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        wait_until_ready(&self.timeout, &self.handle, poll::POLLOUT)?;
        self.handle.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        wait_until_ready(&self.timeout, &self.handle, poll::POLLOUT)?;
        self.handle.flush()
    }
}

impl<H> Clone for TimeoutWriter<H>
    where H: Write + AsRawFd + Clone
{
    fn clone(&self) -> TimeoutWriter<H> {
        TimeoutWriter { handle: self.handle.clone(), ..*self }
    }
}

impl<H> TimeoutWriter<H>
    where H: Write + AsRawFd
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
            timeout: timeout.into().map(duration_to_ms),
            handle: handle,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;
    use std::time::Duration;

    use super::*;

    lazy_static! {
        static ref CRATE_ROOT: PathBuf = {
            env::current_exe().unwrap()
                .parent().unwrap()
                .parent().unwrap()
                .parent().unwrap()
                .parent().unwrap()
                .to_path_buf()
        };
    }

    #[test]
    fn read_regular_file_with_timeout() {
        let original_contents = include_str!("../test_data/regular_file.txt");

        let mut regular_file = CRATE_ROOT.clone();
        regular_file.push("test_data");
        regular_file.push("regular_file.txt");

        let fp = File::open(regular_file).unwrap();
        let mut fp = TimeoutReader::new(fp, Duration::new(5, 0));

        let mut read_contents = String::new();
        fp.read_to_string(&mut read_contents).unwrap();

        assert_eq!(original_contents, read_contents);
    }

    #[test]
    fn read_regular_file_no_timeout() {
        let original_contents = include_str!("../test_data/regular_file.txt");

        let mut regular_file = CRATE_ROOT.clone();
        regular_file.push("test_data");
        regular_file.push("regular_file.txt");

        let fp = File::open(regular_file).unwrap();
        let mut fp = TimeoutReader::new(fp, None);

        let mut read_contents = String::new();
        fp.read_to_string(&mut read_contents).unwrap();

        assert_eq!(original_contents, read_contents);
    }
}
