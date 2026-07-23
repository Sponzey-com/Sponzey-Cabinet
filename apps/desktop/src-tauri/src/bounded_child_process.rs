use std::process::{Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) fn run_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> Result<ExitStatus, &'static str> {
    let mut child = command.spawn().map_err(|_| "CHILD_PROCESS_SPAWN_FAILED")?;
    let deadline = Instant::now() + timeout;

    loop {
        match child.try_wait().map_err(|_| "CHILD_PROCESS_WAIT_FAILED")? {
            Some(status) => return Ok(status),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("CHILD_PROCESS_TIMEOUT");
            }
            None => thread::sleep(Duration::from_millis(10)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run_with_timeout;
    use std::process::Command;
    use std::time::{Duration, Instant};

    #[test]
    fn returns_the_completed_child_status() {
        let mut command = Command::new("/usr/bin/true");

        let status =
            run_with_timeout(&mut command, Duration::from_secs(1)).expect("completed child status");

        assert!(status.success());
    }

    #[test]
    fn kills_a_child_that_exceeds_the_deadline() {
        let mut command = Command::new("/bin/sleep");
        command.arg("5");
        let started = Instant::now();

        let error = run_with_timeout(&mut command, Duration::from_millis(50))
            .expect_err("child must time out");

        assert_eq!(error, "CHILD_PROCESS_TIMEOUT");
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
