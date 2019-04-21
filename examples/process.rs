extern crate timeout_readwrite;

use std::env;
use std::io::Result;
use std::io::{BufRead, BufReader};
use std::process;
use std::time::Duration;

use timeout_readwrite::TimeoutReadExt;

fn each_line<R: BufRead>(rdr: R) -> Result<()> {
    let lines = rdr.lines();

    for rslt_line in lines {
        let line = rslt_line?;
        println!("{}", line);
    }
    Ok(())
}

fn do_command<I: Iterator<Item = String>>(mut args: I) -> Result<()> {
    let cmd = args.next().expect("did not pass a program");
    let mut cmd = process::Command::new(cmd);
    for arg in args {
        cmd.arg(arg);
    }

    let child = cmd
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::null())
        .spawn()
        .expect("spawning did not succeed");

    let stdout = child.stdout.expect("stdout must be there");
    each_line(BufReader::new(stdout.with_timeout(Duration::new(5, 0))))
}

fn main() {
    let args = env::args().skip(1);
    do_command(args).expect("failed to do command");
}
