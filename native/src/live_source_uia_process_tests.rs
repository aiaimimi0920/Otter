//! 真实自有控件→受限MTA child；独立常量oracle、密码排除、取消/进程与容量回收。
use super::*;
use crate::live_source_uia_test_window::Window;
#[test]
fn source_uia_private_mode_and_frame_limits_are_strict() {
    assert!(parse(&[FLAG.into()]).unwrap());
    assert!(!parse(&["--capture".into()]).unwrap());
    for args in [
        vec![FLAG.into(), "--settings".into()],
        vec!["--capture".into(), FLAG.into()],
    ] {
        assert!(parse(&args).is_err());
    }
    for n in [0u32, MAX_WIRE as u32 + 1, u32::MAX] {
        assert!(read_frame(&mut &n.to_le_bytes()[..]).is_err());
    }
}
#[test]
fn source_uia_owned_edit_reads_changes_excludes_password_and_reclaims_child() {
    let window = Window::new();
    let target = Target::bind(window.hwnd).unwrap();
    let (send, recv) = std::sync::mpsc::sync_channel(1);
    let (next, wait) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let mut worker = Worker::start(target, &|| false).unwrap();
        let probe = worker.child.probe().unwrap();
        for _ in 0..2 {
            let values = worker.scan(&|| false).unwrap();
            send.send(values).unwrap();
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        assert!(worker.scan(&|| true).is_err());
        drop(worker);
        assert!(probe.exited());
        assert!(probe.peak_job_bytes().unwrap() <= LIMIT);
    });
    for expected in ["value-seven", "value-eight"] {
        let deadline = Instant::now() + Duration::from_secs(5);
        let samples = loop {
            Window::pump();
            if let Ok(samples) = recv.try_recv() {
                break samples;
            }
            assert!(Instant::now() < deadline, "UIA real fixture deadline");
            std::thread::sleep(Duration::from_millis(1));
        };
        let bytes = serde_json::to_string(&samples).unwrap();
        assert!(!bytes.contains("password-secret"));
        let edits: Vec<_> = samples
            .iter()
            .filter(|s| s.locator.control_type == "Edit")
            .collect();
        assert_eq!(edits.len(), 1, "{samples:?}");
        assert_eq!(edits[0].value.as_ref().unwrap()["value"]["text"], expected);
        assert_eq!(edits[0].value.as_ref().unwrap()["value"]["readOnly"], false);
        window.value("value-eight");
        next.send(()).unwrap();
    }
    while !worker.is_finished() {
        Window::pump();
        std::thread::sleep(Duration::from_millis(1));
    }
    worker.join().unwrap();
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn source_uia_stalled_child_cancellation_discards_pipe_and_reclaims_job() {
    let window = Window::new();
    let target = Target::bind(window.hwnd).unwrap();
    let (pipe, endpoint) = Pipe::pair().unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let child = bounded::start(
        endpoint.into_handle(),
        deadline,
        &|| false,
        bounded::Policy {
            suffix: " --exact live_source_uia::process::tests::stalled_entry --ignored --nocapture",
            maximum: LIMIT,
            pool: &CAPACITY,
            environment: bounded::Environment::SystemRoot,
        },
    )
    .unwrap();
    let probe = child.probe().unwrap();
    let mut worker = Worker {
        pipe,
        child,
        target,
    };
    let start = Instant::now();
    assert!(worker
        .scan(&|| start.elapsed() >= Duration::from_millis(150))
        .is_err());
    assert!(start.elapsed() < Duration::from_secs(1));
    drop(worker);
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
#[ignore = "owned stalled UIA child, only parent cancellation test invokes"]
fn stalled_entry() {
    bounded::require_limits(LIMIT).unwrap();
    std::thread::sleep(Duration::from_secs(60));
    std::process::exit(1);
}
#[test]
#[ignore = "private UIA worker entry, only owned bounded parent invokes"]
fn worker_entry() {
    std::process::exit(if run_inherited().is_ok() { 0 } else { 1 });
}
