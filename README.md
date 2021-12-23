timeout-readwrite
=================

A Rust crate providing Reader and Writer structs that timeout.

[![Build Status](https://travis-ci.org/jcreekmore/timeout-readwrite-rs.svg?branch=master)](https://travis-ci.org/jcreekmore/timeout-readwrite-rs)

Why is this useful when we have async I/O? Take, for instance, the case where you
are interacting with a subprocess in the background. This subprocess is waiting for
some event to occur and, when it does, will output a message to standard out. Your
program is waiting for input to come in over the subprocesses standard out stream.
Perhaps you want to write code like the following that does something interesting
for each line as it comes in, but you do not mind blocking until the next line comes in:

```rust
use std::io::{BufRead, BufReader};
use std::io::Result;
use std::process;

fn each_line<R: BufRead>(rdr: R) -> Result<()> {
    let lines = rdr.lines();

    for rslt_line in lines {
        let line = rslt_line?;
        println!("{}", line);
    }
    Ok(())
}

fn do_command(cmd: &str) -> Result<()> {
    let mut cmd = process::Command::new(cmd);
    let child = cmd.stdout(process::Stdio::piped())
        .stderr(process::Stdio::null())
        .spawn().expect("spawning did not succeed");

    let stdout = child.stdout?;
    each_line(BufReader::new(stdout))
}
```

This works as long as the spawned process does not hang and provides output
at a reasonable rate. If the spawned process hangs, though, your program will
be blocked forever waiting for another line to appear. Ideally, we would like
to be able to timeout the read operation and return an error in that case.
Hence, a TimeoutReader struct to provide that ability. With that, we can
change our `do_command` function like so:

```rust
use std::time::Duration;

fn do_command(cmd: &str) -> Result<()> {
    let mut cmd = process::Command::new(cmd);
    let child = cmd.stdout(process::Stdio::piped())
        .stderr(process::Stdio::null())
        .spawn().expect("spawning did not succeed");

    let stdout = child.stdout?;
    each_line(BufReader::new(TimeoutReader::new(stdout, Duration::new(5, 0))))
}
```

Now, if the program ever has to wait longer than 5 seconds without receiving
data from the subprocess, the read call behind the Lines iterator will fail
with an `ErrorKind::TimeOut`.

### Documentation
[Module documentation with examples](https://docs.rs/timeout-readwrite/)

### Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
timeout-readwrite = "0.3.1"
```

and this to your crate root:

```rust
extern crate timeout_readwrite;
```
