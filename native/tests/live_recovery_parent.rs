#![cfg(windows)]
use otter_native::live_recovery_parent::Parent;
use std::{
    io::Read,
    os::windows::{io::AsHandle, process::CommandExt},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Fixture(Child);
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

#[test]
fn real_child_authenticates_only_same_executable_parent_and_owned_pipe() {
    for (pipe, output_pipe, correct_pid) in [
        (true, true, true),
        (false, true, true),
        (true, false, true),
        (true, true, false),
    ] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0);
        command
            .args([
                "--exact",
                "recovery_parent_child",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("OTTER_RECOVERY_AUTH_FIXTURE", "1")
            .env(
                "OTTER_RECOVERY_AUTH_PARENT",
                if correct_pid {
                    std::process::id()
                } else {
                    u32::MAX
                }
                .to_string(),
            )
            .env(
                "OTTER_RECOVERY_AUTH_ACCEPT",
                if pipe && output_pipe && correct_pid {
                    "1"
                } else {
                    "0"
                },
            )
            .stdin(if pipe { Stdio::piped() } else { Stdio::null() })
            .stdout(if output_pipe {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stderr(Stdio::piped());
        let mut child = Fixture(command.spawn().unwrap());
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < deadline, "recovery auth child timed out");
            std::thread::sleep(Duration::from_millis(10));
        };
        let mut output = String::new();
        if let Some(stdout) = child.0.stdout.take() {
            stdout.take(65536).read_to_string(&mut output).unwrap();
        }
        child
            .0
            .stderr
            .take()
            .unwrap()
            .take(65536)
            .read_to_string(&mut output)
            .unwrap();
        assert!(
            status.success(),
            "pipe={pipe} output_pipe={output_pipe} correct_pid={correct_pid}: {output}"
        );
    }
}

#[test]
#[ignore = "helper launched only by the parent authentication fixture"]
fn recovery_parent_child() {
    assert_eq!(std::env::var("OTTER_RECOVERY_AUTH_FIXTURE").unwrap(), "1");
    let expected = std::env::var("OTTER_RECOVERY_AUTH_PARENT")
        .unwrap()
        .parse()
        .unwrap();
    let accept = std::env::var("OTTER_RECOVERY_AUTH_ACCEPT").unwrap() == "1";
    let input = std::io::stdin();
    let output = std::io::stdout();
    let result = Parent::authenticate(expected, input.as_handle(), output.as_handle());
    assert_eq!(
        result.is_ok(),
        accept,
        "authentication result: {:?}",
        result.as_ref().err()
    );
    if let Ok(parent) = result {
        assert!(!parent.exited().unwrap());
        let _borrowed = parent.handle();
    }
}
