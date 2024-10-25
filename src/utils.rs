// Copyright 2017 Jonathan Creekmore
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use nix::libc::c_int;
use nix::poll;
use std::cmp;
use std::convert::TryFrom;
use std::io::{Error, ErrorKind, Result};
use std::os::fd::AsFd;
use std::slice;
use std::time::Duration;

/// Convert from a duration into milliseconds as the c_int type that poll expects.
/// If the duration exceeds the number of milliseconds that can fit into a c_int,
/// saturate the time to the max_value of c_int.
pub fn duration_to_ms(duration: Duration) -> c_int {
    let secs = cmp::min(duration.as_secs(), c_int::MAX as u64) as c_int;
    let nanos = duration.subsec_nanos() as c_int;

    secs.saturating_mul(1_000).saturating_add(nanos / 1_000_000)
}

/// Wait until `to_fd` receives the poll event from `events`, up to `timeout` length
/// of time.
pub fn wait_until_ready(
    timeout: Option<c_int>,
    fd: &impl AsFd,
    events: poll::PollFlags,
) -> Result<()> {
    if let Some(timeout) = timeout {
        let mut pfd = poll::PollFd::new(fd.as_fd(), events);
        let s = slice::from_mut(&mut pfd);

        let timeout =
            poll::PollTimeout::try_from(timeout).map_err(|e| Error::new(ErrorKind::Other, e))?;

        let retval = poll::poll(s, timeout).map_err(|e| Error::new(ErrorKind::Other, e))?;
        if retval == 0 {
            return Err(Error::new(
                ErrorKind::TimedOut,
                "timed out waiting for fd to be ready",
            ));
        }
    }
    Ok(())
}
