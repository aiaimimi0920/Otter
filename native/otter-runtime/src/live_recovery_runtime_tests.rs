//! Three-process recovery verification: source windows stay in the observer, the registering parent exits, and the independent recovery child remains alive.
use super::*;
use crate::{
    live_recovery_wire::{Record, Restoration},
    live_window_target::{snapshot::Snapshot, Target},
};
use std::{
    io::{Read, Write},
    os::windows::io::{FromRawHandle, OwnedHandle},
};
use windows::{core::w, Win32::UI::WindowsAndMessaging::*};

struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
struct ParentFixture(Child);
impl Drop for ParentFixture {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
struct RecoveryFixture(OwnedHandle);
impl Drop for RecoveryFixture {
    fn drop(&mut self) {
        unsafe {
            let handle = HANDLE(self.0.as_raw_handle());
            if WaitForSingleObject(handle, 0) == WAIT_TIMEOUT {
                // This test parent duplicated the handle from its self-created Child; never guess termination targets from external PIDs.
                let _ = TerminateProcess(handle, 99);
                let _ = WaitForSingleObject(handle, 5000);
            }
        }
    }
}
fn pump() {
    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

#[test]
#[ignore = "explicit owned-window three-process recovery runtime gate"]
fn actual_parent_exit_restores_surviving_window_and_eof_does_not_restore_early() {
    for abrupt in [false, true] {
        let window = Window(unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("recovery runtime fixture"),
                WS_POPUP,
                20,
                30,
                40,
                50,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        });
        let before = Snapshot::read(&Target::bind(window.0).unwrap())
            .unwrap()
            .recovery_record()
            .unwrap();
        run_parent(&before, abrupt);
        let after = Snapshot::read(&Target::bind(window.0).unwrap())
            .unwrap()
            .recovery_record()
            .unwrap();
        assert_eq!(after, before);
        assert!(!unsafe { IsWindowVisible(window.0) }.as_bool());
        println!("native-recovery-runtime: abrupt={abrupt} ack-before-change=true eof-held=true restored=true child-exited=true");
    }
}

#[test]
#[ignore = "explicit owned-menu three-process popup layer recovery gate"]
fn actual_parent_exit_releases_surviving_menu_layer_without_hiding_or_moving_it() {
    use crate::live_window_target::recovery::popup_layer::fixture::Fixture;
    for abrupt in [false, true] {
        let fixture = Fixture::with_dll();
        let lease = fixture.show();
        let before = Snapshot::read(&Target::bind(fixture.popup.0).unwrap()).unwrap();
        run_parent(&fixture.record(lease), abrupt);
        let after = Snapshot::read(&Target::bind(fixture.popup.0).unwrap()).unwrap();
        assert!(!fixture.topmost());
        assert_eq!(before.rect, after.rect);
        assert_eq!(before.visible, after.visible);
        assert_eq!(before.extended_style, after.extended_style);
        assert!(
            lease.valid(),
            "layer recovery must not revoke otherwise valid input scope"
        );
    }
}

fn run_parent(before: &Record, abrupt: bool) {
    let mut parent = ParentFixture(
        Command::new(std::env::current_exe().unwrap())
            .creation_flags(CREATE_NO_WINDOW.0)
            .args([
                "--exact",
                "live_recovery_client::runtime_tests::registration_parent",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(
                "OTTER_RECOVERY_RUNTIME_RECORD",
                serde_json::to_string(before).unwrap(),
            )
            .env(
                "OTTER_RECOVERY_RUNTIME_OBSERVER",
                std::process::id().to_string(),
            )
            .env(
                "OTTER_RECOVERY_RUNTIME_ABRUPT",
                if abrupt { "1" } else { "0" },
            )
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        pump();
        if let Some(status) = parent.0.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() >= deadline {
            parent.0.kill().unwrap();
            parent.0.wait().unwrap();
            break None;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    // Read only existing bytes left by the exited parent; waiting for EOF must not block the window message pump.
    // If descendants accidentally inherit output handles, EOF may arrive after parent exit.
    let mut stdout = parent.0.stdout.take().unwrap();
    let mut available = 0;
    unsafe {
        windows::Win32::System::Pipes::PeekNamedPipe(
            HANDLE(stdout.as_raw_handle()),
            None,
            0,
            None,
            Some(&mut available),
            None,
        )
        .unwrap();
    }
    assert!(available <= 65536);
    let mut bytes = vec![0; available as usize];
    stdout.read_exact(&mut bytes).unwrap();
    let output = String::from_utf8(bytes).unwrap();
    let raw = output
        .lines()
        .find_map(|line| {
            line.split_once("RECOVERY_FIXTURE_HANDLE=")
                .map(|(_, value)| value)
        })
        .expect("Fixture must transfer owned recovery handle")
        .parse::<usize>()
        .unwrap();
    // Accept only handles transferred by this controlled subprocess into the current process through DuplicateHandle, not external PIDs.
    let recovery = RecoveryFixture(unsafe { OwnedHandle::from_raw_handle(raw as *mut _) });
    assert!(output.contains("RECOVERY_FIXTURE_EOF_HELD"), "{output}");
    assert_eq!(
        status.expect("Registration fixture timed out").code(),
        Some(if abrupt { 23 } else { 0 })
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        pump();
        let result = unsafe { WaitForSingleObject(HANDLE(recovery.0.as_raw_handle()), 0) };
        if result == WAIT_OBJECT_0 {
            break;
        }
        assert_eq!(result, WAIT_TIMEOUT);
        assert!(Instant::now() < deadline, "Recovery child did not finish");
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut exit_code = 0;
    unsafe {
        GetExitCodeProcess(HANDLE(recovery.0.as_raw_handle()), &mut exit_code).unwrap();
    }
    assert_eq!(exit_code, 0);
}

#[test]
#[ignore = "private registration parent for the three-process fixture"]
fn registration_parent() {
    let record: Record =
        serde_json::from_str(&std::env::var("OTTER_RECOVERY_RUNTIME_RECORD").unwrap()).unwrap();
    let observer = std::env::var("OTTER_RECOVERY_RUNTIME_OBSERVER")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert_eq!(observer, record.process_id);
    let mut client = Client::start_command(super::tests::fixture("serve")).unwrap();
    unsafe {
        let observer = OwnedHandle::from_raw_handle(
            OpenProcess(PROCESS_DUP_HANDLE, false, observer).unwrap().0,
        );
        let mut transferred = HANDLE::default();
        DuplicateHandle(
            GetCurrentProcess(),
            HANDLE(client.child.as_raw_handle()),
            HANDLE(observer.as_raw_handle()),
            &mut transferred,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
        .unwrap();
        println!("RECOVERY_FIXTURE_HANDLE={}", transferred.0 as usize);
        std::io::stdout().flush().unwrap();
    }
    assert_eq!(
        client
            .request(Operation::Register {
                id: 1,
                record: record.clone()
            })
            .unwrap(),
        Status::Accepted
    );
    let hwnd = HWND(record.hwnd as usize as *mut _);
    // A real ACK has arrived; retain the request owner without proactively restoring, so the independent child performs cleanup after exit.
    let mut layer_owner = None;
    let altered = match &record.restoration {
        Restoration::PopupLayer(layer) => {
            let original_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
            use crate::live_window_target::recovery::popup_layer::{fixture::settle, Recovery};
            let recovery = Recovery::bind(Target::bind(hwnd).unwrap(), layer).unwrap();
            settle(|| recovery.acquire());
            layer_owner = Some(recovery);
            original_style | WS_EX_TOPMOST.0 as isize
        }
        Restoration::Window(state) => {
            let snapshot = Snapshot::read(&Target::bind(hwnd).unwrap()).unwrap();
            snapshot.apply_hidden().unwrap();
            assert_eq!(
                Snapshot::read(&Target::bind(hwnd).unwrap())
                    .unwrap()
                    .layered
                    .unwrap()
                    .alpha,
                1
            );
            state.extended_style as isize | WS_EX_TRANSPARENT.0 as isize | WS_EX_LAYERED.0 as isize
        }
    };
    assert_eq!(unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) }, altered);
    drop(client);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) }, altered);
    println!("RECOVERY_FIXTURE_EOF_HELD");
    std::io::stdout().flush().unwrap();
    if std::env::var("OTTER_RECOVERY_RUNTIME_ABRUPT").unwrap() == "1" {
        std::process::exit(23);
    }
    drop(layer_owner); // Drop only revokes requests; local restoration before parent exit must not substitute for child acceptance testing.
}
