// Copyright 2017 Jonathan Creekmore
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(unix)]
use nix::poll::PollFlags;
use std::io::Read;
use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
use std::os::raw::c_int;
use std::time::Duration;
#[cfg(windows)]
use winapi::um;

use super::utils;

/// The `TimeoutReader` struct adds read timeouts to any reader.
///
/// The `read` call on a `Read` instance will block forever until data is available.
/// A `TimeoutReader` will wait until data is available, up until an optional timeout,
/// before actually performing the `read` operation.
///
/// If any `Read` operation times out, the method called will return
/// an `io::ErrorKind::TimedOut` variant as the value of `io::Error`. All other
/// error values that would normally be produced by the underlying implementation
/// of the `Read` trait could also be produced by the `TimeoutReader`.
pub struct TimeoutReader<H> {
    timeout: Option<c_int>,
    handle: H,
}

#[cfg(unix)]
impl<H> Read for TimeoutReader<H>
where
    H: Read + AsRawFd,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        utils::wait_until_ready(self.timeout,
            &self.handle, PollFlags::POLLIN)?;
        self.handle.read(buf)
    }
}

#[cfg(windows)]
impl<H> Read for TimeoutReader<H>
where
    H: Read + AsRawHandle,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if let Some(timeout) = self.timeout {
            let handle = self.handle.as_raw_handle();
            let mut timeouts = unsafe { ::std::mem::zeroed::<um::winbase::COMMTIMEOUTS>() };
            timeouts.ReadTotalTimeoutConstant = timeout as u32;

            unsafe { um::commapi::SetCommTimeouts(handle, &mut timeouts) };
        }
        self.handle.read(buf)
    }
}

impl<H> Seek for TimeoutReader<H>
where
    H: Seek,
{
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.handle.seek(pos)
    }
}

#[cfg(unix)]
impl<H> AsRawFd for TimeoutReader<H>
where
    H: Read + AsRawFd,
{
    fn as_raw_fd(&self) -> c_int {
        self.handle.as_raw_fd()
    }
}

#[cfg(windows)]
impl<H> AsRawHandle for TimeoutReader<H>
where
    H: Read + AsRawHandle,
{
    fn as_raw_handle(&self) -> ::std::os::windows::io::RawHandle {
        self.handle.as_raw_handle()
    }
}

impl<H> Clone for TimeoutReader<H>
where
    H: Clone,
{
    fn clone(&self) -> TimeoutReader<H> {
        TimeoutReader {
            handle: self.handle.clone(),
            ..*self
        }
    }
}

impl<H> TimeoutReader<H>
where
    H: Read,
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
            timeout: timeout.into().map(utils::duration_to_ms),
            handle: handle,
        }
    }
}

pub trait TimeoutReadExt<H> {
    fn with_timeout<T: Into<Option<Duration>>>(self, timeout: T) -> TimeoutReader<H>;
}

impl<H> TimeoutReadExt<H> for H
where
    H: Read
{
    fn with_timeout<T: Into<Option<Duration>>>(self, timeout: T) -> TimeoutReader<H> {
        TimeoutReader::new(self, timeout)
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
            env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
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

    #[test]
    fn read_regular_file_with_timeout_extension_trait() {
        let original_contents = include_str!("../test_data/regular_file.txt");

        let mut regular_file = CRATE_ROOT.clone();
        regular_file.push("test_data");
        regular_file.push("regular_file.txt");

        let fp = File::open(regular_file).unwrap();
        let mut fp = fp.with_timeout(Duration::new(5, 0));

        let mut read_contents = String::new();
        fp.read_to_string(&mut read_contents).unwrap();

        assert_eq!(original_contents, read_contents);
    }

    #[test]
    fn read_tcp_read_with_timeout_extension_trait() {
        use std::net::{TcpListener, TcpStream};
        use std::thread::spawn;
        use std::sync::{Arc, Barrier};

        const ADDR: &str = "127.0.0.1:8080";

        let bind_done = Arc::new(Barrier::new(2));
        let wait_bind_done = bind_done.clone();

        let timeout_done = Arc::new(Barrier::new(2));
        let wait_timeout_done = timeout_done.clone();

        let _h = spawn(move || {
            let listener = TcpListener::bind(ADDR).unwrap();
            bind_done.wait();
            let _stream = listener.accept().unwrap();
            wait_timeout_done.wait();
        });

        wait_bind_done.wait();

        let fp = TcpStream::connect(ADDR).unwrap();
        let mut fp = fp.with_timeout(Duration::new(5, 0));

        let mut read_contents = String::new();
        let value = fp.read_to_string(&mut read_contents); 
        timeout_done.wait();

        if let Err(e) = value {
            assert_eq!(e.kind(), ::std::io::ErrorKind::TimedOut);
        } else {
            assert!(false);
        }
    }
}
