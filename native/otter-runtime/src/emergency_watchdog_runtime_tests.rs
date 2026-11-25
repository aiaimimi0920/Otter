//! 真实同EXE认证/握手/进程回收；确定性键位仅存在于cfg(test)牺牲进程。
use super::*;
use std::{
    io::{Read, Write},
    os::windows::{
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
        process::CommandExt,
    },
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use windows::Win32::UI::WindowsAndMessaging::{
    PeekMessageW, PostThreadMessageW, MSG, PM_NOREMOVE, WM_INPUT,
};

// 仅唤醒本夹具 watchdog 的私有消息队列，不发送系统键盘输入。
// 合成键位仍经过生产事件循环、Gate 与父进程认证，避免测到旧轮询实现。
struct FixtureWake {
    stopped: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl FixtureWake {
    fn new() -> Self {
        let thread_id = unsafe {
            let _ = PeekMessageW(&mut MSG::default(), None, 0, 0, PM_NOREMOVE);
            GetCurrentThreadId()
        };
        let stopped = Arc::new(AtomicBool::new(false));
        let child_stopped = stopped.clone();
        let thread = std::thread::spawn(move || {
            while !child_stopped.load(Ordering::Acquire) {
                unsafe {
                    let _ = PostThreadMessageW(thread_id, WM_INPUT, WPARAM(0), LPARAM(0));
                }
                std::thread::sleep(Duration::from_millis(u64::from(SAMPLE_MS)));
            }
        });
        Self {
            stopped,
            thread: Some(thread),
        }
    }
}
impl Drop for FixtureWake {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap();
    }
}

struct ChildOwner(Child);
impl Drop for ChildOwner {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
struct WatchdogHandle(OwnedHandle);
impl Drop for WatchdogHandle {
    fn drop(&mut self) {
        unsafe {
            let handle = HANDLE(self.0.as_raw_handle());
            if WaitForSingleObject(handle, 0) == WAIT_TIMEOUT {
                // 只回收本夹具由DuplicateHandle转交的自建child，不按外部PID打开。
                let _ = TerminateProcess(handle, 99);
                let _ = WaitForSingleObject(handle, 5000);
            }
        }
    }
}
fn fixture(entry: &str, mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            &format!("emergency_watchdog::tests::runtime::{entry}"),
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("OTTER_EMERGENCY_FIXTURE_MODE", mode)
        .env(
            "OTTER_EMERGENCY_FIXTURE_PARENT",
            std::process::id().to_string(),
        );
    command
}
fn wait(handle: std::os::windows::io::BorrowedHandle<'_>, ms: u32) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), ms) },
        WAIT_OBJECT_0
    );
}

#[test]
fn owner_handshakes_and_drop_reaps_only_its_idle_child() {
    let owner = Owner::start_command(fixture("watchdog_fixture", "idle")).unwrap();
    let handle = owner.handle().try_clone_to_owned().unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 0) },
        WAIT_TIMEOUT
    );
    drop(owner);
    wait(handle.as_handle(), 5000);
}

#[test]
fn failed_authentication_and_missing_ready_never_arm() {
    for mode in ["bad-parent", "no-ready"] {
        assert!(Owner::start_command(fixture("watchdog_fixture", mode)).is_err());
    }
}

#[test]
fn deterministic_chords_terminate_only_authenticated_sacrificial_parent_and_child_exits() {
    for mode in ["backup", "triple", "parent-exit", "shutdown"] {
        let started = Instant::now();
        let mut parent = ChildOwner(
            fixture("parent_fixture", mode)
                .creation_flags(CREATE_NO_WINDOW.0)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let status = unsafe {
            WaitForSingleObject(
                HANDLE(parent.0.as_raw_handle()),
                if mode == "shutdown" { 40000 } else { 15000 },
            )
        };
        // 仅读取已到达字节，失败时不能因后代继承stdout而无限等EOF。
        let mut output = parent.0.stdout.take().unwrap();
        let mut available = 0;
        unsafe {
            windows::Win32::System::Pipes::PeekNamedPipe(
                HANDLE(output.as_raw_handle()),
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
        output.read_exact(&mut bytes).unwrap();
        let output = String::from_utf8(bytes).unwrap();
        let raw = output
            .lines()
            .find_map(|row| {
                row.split_once("EMERGENCY_FIXTURE_HANDLE=")
                    .map(|(_, value)| value)
            })
            .expect("Owned fixture must transfer child handle")
            .parse::<usize>()
            .unwrap();
        let watchdog = WatchdogHandle(unsafe { OwnedHandle::from_raw_handle(raw as *mut _) });
        assert_eq!(
            status, WAIT_OBJECT_0,
            "Sacrificial parent timed out: {mode}"
        );
        if mode == "shutdown" {
            assert!(started.elapsed() >= Duration::from_secs(30));
        }
        assert_eq!(
            parent.0.wait().unwrap().code(),
            Some(if mode == "parent-exit" { 23 } else { 0 })
        );
        wait(watchdog.0.as_handle(), 5000);
        let mut exit = 0;
        unsafe {
            GetExitCodeProcess(HANDLE(watchdog.0.as_raw_handle()), &mut exit).unwrap();
        }
        assert_eq!(exit, 0, "{mode}");
        println!("native-emergency-fixture: mode={mode} parent-exited=true watchdog-exited=true");
    }
}

#[test]
#[ignore = "private authenticated child, only launched by owned emergency fixtures"]
fn watchdog_fixture() {
    let mode = std::env::var("OTTER_EMERGENCY_FIXTURE_MODE").unwrap();
    if mode == "no-ready" {
        return;
    }
    let parent: u32 = std::env::var("OTTER_EMERGENCY_FIXTURE_PARENT")
        .unwrap()
        .parse()
        .unwrap();
    let pipe = Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned().unwrap()).unwrap();
    if mode == "bad-parent" {
        assert!(run(std::process::id(), pipe, || panic!(
            "Unauthenticated input sampled"
        ))
        .is_err());
        return;
    }
    assert!(["idle", "parent-exit", "triple", "backup", "shutdown"].contains(&mode.as_str()));
    let _wake = matches!(mode.as_str(), "backup" | "triple").then(FixtureWake::new);
    let mut sample = 0u32;
    run(parent, pipe, || {
        sample += 1;
        if sample <= 32 {
            return Keys::default();
        }
        match mode.as_str() {
            "backup" => Keys {
                backup: true,
                ..Keys::default()
            },
            "triple" => {
                let pressed = (sample - 33) % 4 < 2;
                Keys {
                    escape: pressed,
                    delete: pressed,
                    backup: false,
                }
            }
            _ => Keys::default(),
        }
    })
    .unwrap();
}

#[test]
#[ignore = "private sacrificial parent, only launched by owned emergency fixtures"]
fn parent_fixture() {
    let mode = std::env::var("OTTER_EMERGENCY_FIXTURE_MODE").unwrap();
    assert!(["backup", "triple", "parent-exit", "shutdown"].contains(&mode.as_str()));
    let observer: u32 = std::env::var("OTTER_EMERGENCY_FIXTURE_PARENT")
        .unwrap()
        .parse()
        .unwrap();
    let mut owner = Owner::start_command(fixture("watchdog_fixture", &mode)).unwrap();
    unsafe {
        let observer = OwnedHandle::from_raw_handle(
            OpenProcess(PROCESS_DUP_HANDLE, false, observer).unwrap().0,
        );
        let mut transferred = HANDLE::default();
        DuplicateHandle(
            GetCurrentProcess(),
            HANDLE(owner.handle().as_raw_handle()),
            HANDLE(observer.as_raw_handle()),
            &mut transferred,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
        .unwrap();
        println!("EMERGENCY_FIXTURE_HANDLE={}", transferred.0 as usize);
        std::io::stdout().flush().unwrap();
    }
    if mode == "parent-exit" {
        std::process::exit(23);
    }
    if mode == "shutdown" {
        owner.begin_shutdown().unwrap();
    }
    // 故意不泵UI或处理键盘：独立child必须能终止一个阻塞的原父进程。
    std::thread::sleep(Duration::from_secs(if mode == "shutdown" {
        35
    } else {
        10
    }));
    panic!("Deterministic emergency failed to terminate sacrificial parent");
}
