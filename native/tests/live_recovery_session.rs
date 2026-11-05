#![cfg(windows)]
//! 真实同 executable 父子进程协议；辅助入口仅由本测试创建并接收已知 overlapped 管道。
use otter_native::{
    live_recovery_handshake::{read_ready, write_hello},
    live_recovery_pipe::Pipe,
    live_recovery_wire::{
        read_ack, write_request, Command as Operation, Record, Request, Status, SCHEMA,
    },
};
use std::{
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
fn authenticated_child_completes_real_session_and_rejects_unbound_source() {
    let (mut server, client) = Pipe::pair().unwrap();
    let mut child = Fixture(
        Command::new(std::env::current_exe().unwrap())
            .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0)
            .args([
                "--exact",
                "recovery_session_child",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("OTTER_RECOVERY_SESSION_FIXTURE", "1")
            .env(
                "OTTER_RECOVERY_SESSION_PARENT",
                std::process::id().to_string(),
            )
            .stdin(Stdio::from(client.into_handle()))
            // 测试框架 stdout 不参与协议，避免其标题/结果污染私有 duplex 管道。
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut channel = server.io(
        Some(Instant::now() + Duration::from_secs(10)),
        Some(child.0.as_handle()),
    );
    write_hello(&mut channel, [9; 16]).unwrap();
    read_ready(&mut channel, [9; 16]).unwrap();
    // 结构合法但不可能绑定的 HWND，不创建或更改真实外部窗口。
    let record = Record {
        hwnd: u64::MAX,
        process_id: std::process::id(),
        thread_id: 1,
        process_created: 1,
        rect: [0, 0, 32, 32],
        placement_flags: 0,
        show_command: 1,
        min_position: [0, 0],
        max_position: [0, 0],
        normal_position: [0, 0, 32, 32],
        extended_style: 0,
        layered: None,
        iconic: false,
        visible: true,
    };
    for (sequence, command, expected) in [
        (1, Operation::Register { id: 1, record }, Status::Rejected),
        (2, Operation::Shutdown, Status::Accepted),
    ] {
        let request = Request {
            schema: SCHEMA,
            session: [9; 16],
            sequence,
            command,
        };
        write_request(&mut channel, &request).unwrap();
        assert_eq!(
            read_ack(&mut channel, [9; 16], sequence).unwrap().status,
            expected
        );
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(
                status.success(),
                "Recovery session fixture failed: {status}"
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Recovery session fixture timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[ignore = "helper launched only by authenticated_child_completes_real_session_and_rejects_unbound_source"]
fn recovery_session_child() {
    assert_eq!(
        std::env::var("OTTER_RECOVERY_SESSION_FIXTURE").unwrap(),
        "1"
    );
    let parent = std::env::var("OTTER_RECOVERY_SESSION_PARENT")
        .unwrap()
        .parse()
        .unwrap();
    otter_native::live_recovery_runtime::run_inherited(parent).unwrap();
}

#[test]
fn production_cli_rejects_invalid_or_unpiped_private_mode_without_desktop_startup() {
    use otter_native::live_recovery_runtime::FLAG;
    for args in [
        vec![FLAG.to_string(), "0".to_string()],
        vec![FLAG.to_string(), std::process::id().to_string()],
        vec![FLAG.to_string(), "1".to_string(), "--capture".to_string()],
    ] {
        let mut child = Fixture(
            Command::new(env!("CARGO_BIN_EXE_otter-native"))
                .creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW.0)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                assert!(!status.success());
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Private mode must reject before desktop startup"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
