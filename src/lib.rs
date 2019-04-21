// Copyright 2017 Jonathan Creekmore
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg(unix)]

//! Provides `TimeoutReader` and `TimeoutWriter` structs to time out reads and
//! writes, respectively. `TimeoutReader` implements `Read` and `TimeoutWriter`
//! implements `Write`. If any operation times out, the method called will return
//! an `io::ErrorKind::TimedOut` variant as the value of `io::Error`. All other
//! error values that would normally be produced by the underlying implementation
//! of the `Read` or `Write` trait could also be produced by the `TimeoutReader` and
//! `TimeoutWriter` structs.
//!
//! # Example: read from a process with a 5-second timeout
//!
//! Given a process that writes to standard out, read from the output once it is there,
//! but fail if you have to wait longer than 5-seconds for data to be present on standard
//! out.
//!
//! ```rust
//! use std::io::{ErrorKind, Read, Result};
//! use std::process;
//! use std::time::Duration;
//! use timeout_readwrite::TimeoutReader;
//!
//! fn read_command(mut cmd: process::Command) -> Result<String> {
//!     let child = cmd.stdout(process::Stdio::piped())
//!        .stderr(process::Stdio::null())
//!        .spawn()
//!        .expect("spawing did not succeed");
//!     let stdout = child.stdout.expect("stdout must be there");
//!
//!     let mut data = String::new();
//!     let mut rdr = TimeoutReader::new(stdout, Duration::new(5, 0));
//!     rdr.read_to_string(&mut data)?;
//!     Ok(data)
//! }
//!
//! match read_command(process::Command::new("ls")) {
//!   Ok(value) => { print!("{}", value); },
//!   Err(ref e) if e.kind() == ErrorKind::TimedOut => { println!("timed out!"); },
//!   Err(ref e) => { println!("failed reading with {}", e); },
//! }
//! ```
//!
//! # Example: use the TimeoutReadExt trait
//!
//! Use the TimeoutReadExt trait to provide a simple helper method to creating a TimeoutReader.
//!
//! ```rust
//! use std::io::{ErrorKind, Read, Result};
//! use std::process;
//! use std::time::Duration;
//! use timeout_readwrite::TimeoutReadExt;
//!
//! fn read_command(mut cmd: process::Command) -> Result<String> {
//!     let child = cmd.stdout(process::Stdio::piped())
//!        .stderr(process::Stdio::null())
//!        .spawn()
//!        .expect("spawing did not succeed");
//!     let stdout = child.stdout.expect("stdout must be there");
//!
//!     let mut data = String::new();
//!     stdout.with_timeout(Duration::new(5, 0)).read_to_string(&mut data)?;
//!     Ok(data)
//! }
//!
//! match read_command(process::Command::new("ls")) {
//!   Ok(value) => { print!("{}", value); },
//!   Err(ref e) if e.kind() == ErrorKind::TimedOut => { println!("timed out!"); },
//!   Err(ref e) => { println!("failed reading with {}", e); },
//! }
//! ```

#[cfg(test)]
#[macro_use]
extern crate lazy_static;
extern crate nix;

mod utils;

pub mod reader;
pub use reader::{TimeoutReadExt, TimeoutReader};

pub mod writer;
pub use writer::{TimeoutWriteExt, TimeoutWriter};
