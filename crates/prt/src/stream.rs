//! JSON streaming mode: `prt --json` outputs NDJSON to stdout.
//!
//! Each line is a complete JSON object representing one [`PortEntry`].
//! A new batch of entries is printed every [`TICK_RATE`] seconds.

use anyhow::Result;
use prt_core::core::scanner;
use prt_core::model::TICK_RATE;
use std::io::{self, BufWriter, Write};

/// Run the JSON streaming loop. Never returns unless interrupted
/// or a write error (SIGPIPE) occurs.
pub fn run_json_stream() -> Result<()> {
    // Ignore SIGPIPE so broken-pipe surfaces as io::ErrorKind::BrokenPipe
    // instead of killing the process (e.g. `prt --json | head -5`).
    #[cfg(unix)]
    unsafe {
        nix::sys::signal::signal(
            nix::sys::signal::Signal::SIGPIPE,
            nix::sys::signal::SigHandler::SigIgn,
        )
        .ok();
    }

    let mut stdout = BufWriter::new(io::stdout().lock());
    loop {
        let entries = scanner::scan()?;
        for entry in &entries {
            match serde_json::to_writer(&mut stdout, entry) {
                Ok(()) => {}
                Err(e) if is_broken_pipe(&e) => return Ok(()),
                Err(e) => return Err(e.into()),
            }
            if let Err(e) = stdout.write_all(b"\n") {
                if e.kind() == io::ErrorKind::BrokenPipe {
                    return Ok(());
                }
                return Err(e.into());
            }
        }
        if let Err(e) = stdout.flush() {
            if e.kind() == io::ErrorKind::BrokenPipe {
                return Ok(());
            }
            return Err(e.into());
        }
        std::thread::sleep(TICK_RATE);
    }
}

fn is_broken_pipe(e: &serde_json::Error) -> bool {
    if let Some(io_err) = e.io_error_kind() {
        io_err == io::ErrorKind::BrokenPipe
    } else {
        false
    }
}
