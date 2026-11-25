//! Local browser-scroll protocol and standalone Edge integration entry point; never automatically discover or control user browsers.
use super::*;
#[test]
fn only_startup_pipe_timeout_is_a_pre_action_transition() {
    let startup = start_error(io::Error::new(
        io::ErrorKind::TimedOut,
        "handshake deadline",
    ));
    assert_eq!(startup.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(startup.to_string(), "handshake deadline");
    for kind in [
        io::ErrorKind::PermissionDenied,
        io::ErrorKind::BrokenPipe,
        io::ErrorKind::InvalidData,
    ] {
        assert_eq!(start_error(io::Error::new(kind, "rejected")).kind(), kind);
    }
}
#[test]
fn navigation_query_timeout_is_recoverable_only_before_any_action() {
    use windows::{
        core::{Error, HRESULT},
        Win32::Foundation::E_ACCESSDENIED,
    };
    for code in [0x80131505_u32, 0x80040201, 0x800705B4] {
        let error = Error::from(HRESULT(code as i32));
        assert_eq!(
            response_error("query", process::failure_reply(&error, false)).kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(
            response_error("action", process::failure_reply(&error, true)).kind(),
            io::ErrorKind::Other
        );
    }
    let denied = Error::from(E_ACCESSDENIED);
    assert_eq!(
        response_error("query", process::failure_reply(&denied, false)).kind(),
        io::ErrorKind::Other
    );
    // Even if the child marks a query transition, do not turn permission errors into recoverable states.
    let mut reply = process::failure_reply(&denied, false);
    reply[0] = 3;
    assert_eq!(response_error("query", reply).kind(), io::ErrorKind::Other);
}
#[test]
fn wheel_residual_preserves_signed_ticks_and_axes() {
    let mut axes = [0; 2];
    assert_eq!(accumulate(&mut axes[0], -30).unwrap(), 0);
    assert_eq!(accumulate(&mut axes[1], 120).unwrap(), 1);
    assert_eq!(accumulate(&mut axes[0], -100).unwrap(), -1);
    assert_eq!(axes, [-10, 0]);
    assert_eq!(accumulate(&mut axes[0], 130).unwrap(), 1);
    assert_eq!(axes, [0, 0]);
    for delta in [0, 1201, -1201, i32::MIN] {
        assert!(accumulate(&mut axes[0], delta).is_err());
        assert_eq!(axes[0], 0);
    }
}
#[test]
fn private_worker_rejects_mixed_arguments() {
    use std::ffi::OsString;
    assert!(!process::parse(&[]).unwrap());
    assert!(process::parse(&[process::FLAG.into()]).unwrap());
    assert!(process::parse(&[process::FLAG.into(), OsString::from("--settings")]).is_err());
}

#[test]
#[ignore = "Requires owned Edge HWND/PID with tests/fixtures/live-browser-scroll.html"]
fn owned_edge_backend() {
    use crate::live_input::{Controller, Event};
    use crate::live_window_target::input_backend::Backend;
    use std::io::BufRead;
    use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::GetWindowTextW};
    let hwnd: usize = std::env::var("OTTER_TEST_EDGE_HWND")
        .unwrap()
        .parse()
        .unwrap();
    let pid: u32 = std::env::var("OTTER_TEST_EDGE_PID")
        .unwrap()
        .parse()
        .unwrap();
    let target = Target::bind(HWND(hwnd as *mut _)).unwrap();
    assert_eq!(target.process_id(), pid);
    let mut title = [0u16; 512];
    let n = unsafe { GetWindowTextW(target.hwnd(), &mut title) } as usize;
    assert!(String::from_utf16_lossy(&title[..n]).starts_with("Otter Live Input Probe"));
    let mut controller =
        Controller::new(Backend::new_local_in_test_harness(target.clone(), None).unwrap());
    controller.enable().unwrap();
    println!("OTTER_EDGE_READY");
    std::io::stdout().flush().unwrap();
    for (index, line) in std::io::stdin().lock().lines().enumerate() {
        let line = line.unwrap();
        if line == "exit" {
            break;
        }
        let values: Vec<_> = line.split_whitespace().collect();
        assert_eq!(values.len(), 4);
        let result = controller.dispatch(
            index as u64 + 1,
            Event::Wheel {
                point: [values[0].parse().unwrap(), values[1].parse().unwrap()],
                delta: values[2].parse().unwrap(),
                horizontal: values[3] == "1",
            },
        );
        println!("OTTER_EDGE_RESULT {result:?}");
        std::io::stdout().flush().unwrap();
    }
    controller.cancel();
    assert_eq!(controller.last_cleanup().failed, 0);
    drop(controller);
    let worker = Worker::start_in_test_harness(target).unwrap();
    let probe = worker.child.probe().unwrap();
    assert!(probe.peak_job_bytes().unwrap() <= LIMIT);
    drop(worker);
    assert!(
        probe.exited(),
        "Browser scroll child must not survive its owner"
    );
}
