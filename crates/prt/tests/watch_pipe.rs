use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

fn wait_for_exit(mut child: Child) -> Output {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "watch did not exit: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn watch_exits_cleanly_when_stdout_is_closed() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_prt"))
        .args(["watch", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = wait_for_exit(child);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn watch_reports_other_stdout_errors() {
    let output = wait_for_exit(
        Command::new(env!("CARGO_BIN_EXE_prt"))
            .args(["watch", "1"])
            .stdout(
                std::fs::File::options()
                    .write(true)
                    .open("/dev/full")
                    .unwrap(),
            )
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("os error 28"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
