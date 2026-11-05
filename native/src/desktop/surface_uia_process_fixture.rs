//! 独立测试进程 owner：仅接受自启进程的 HWND，超时/失败仅清理持有的 Child。
use super::*;
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc::{sync_channel, Receiver},
    thread::JoinHandle,
};
pub(in crate::desktop::surface_window) struct Fixture {
    child: Child,
    reader: Option<JoinHandle<()>>,
    events: Receiver<String>,
    pub hwnd: HWND,
}
impl Fixture {
    pub fn start() -> Self {
        Self::start_mode("default")
    }
    pub fn start_select() -> Self {
        Self::start_mode("select")
    }
    pub fn start_range() -> Self {
        Self::start_mode("range")
    }
    pub fn start_foreground() -> Self {
        // 只借独立输入队列，不要求或伪造 UIA action；仍走原 Child/管道/超时清理。
        Self::start_mode("foreground")
    }
    fn start_mode(mode: &str) -> Self {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "desktop::surface_window::uia::process_tests::fixture::uia_process_fixture_child",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("OTTER_NATIVE_UIA_CHILD", "1")
            .env("OTTER_NATIVE_UIA_MODE", mode)
            .env(
                "OTTER_NATIVE_UIA_PARENT_PID",
                std::process::id().to_string(),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let output = child.stdout.take().unwrap();
        let (sender, events) = sync_channel(4);
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(output)
                .take(64 * 1024)
                .lines()
                .map_while(std::result::Result::ok)
            {
                if let Some((_, value)) = line.split_once("OTTER_UIA_FIXTURE=") {
                    let _ = sender.try_send(value.trim().to_owned());
                }
            }
        });
        let mut owner = Self {
            child,
            reader: Some(reader),
            events,
            hwnd: HWND::default(),
        };
        let ready = owner
            .events
            .recv_timeout(Duration::from_secs(10))
            .expect("UIA child not ready");
        owner.hwnd = HWND(usize::from_str_radix(&ready, 16).unwrap() as *mut _);
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(owner.hwnd, Some(&mut pid));
        }
        assert_eq!(pid, owner.child.id(), "refusing non-fixture HWND");
        owner
    }
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
    pub fn foreground(&mut self) {
        assert!(self.child.try_wait().unwrap().is_none());
        unsafe {
            let mut owner = 0;
            GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut owner));
            assert_eq!(
                owner,
                std::process::id(),
                "foreground donor must be this test"
            );
            // 前台父进程只授权仍持有的 Child；不使用 ASFW_ANY 或替外部进程抢前台。
            AllowSetForegroundWindow(self.child.id()).unwrap();
        }
        // 只对持有的 Child 发一次命令；目标线程完成本地激活后确认，不在父线程轮询/重试。
        self.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"foreground\n")
            .unwrap();
        assert_eq!(
            self.events
                .recv_timeout(Duration::from_secs(5))
                .expect("foreground child did not acknowledge activation"),
            "foreground"
        );
    }
    pub fn accepted(&self) {
        assert_eq!(
            self.events
                .recv_timeout(Duration::from_secs(5))
                .expect("UIA action not accepted"),
            "accepted"
        );
    }
    pub fn return_foreground(&mut self) {
        assert!(self.child.try_wait().unwrap().is_none());
        // 当前前台的自有 Child 归还父进程的激活许可，父线程仍自行完成实际激活。
        self.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"return\n")
            .unwrap();
        assert_eq!(
            self.events
                .recv_timeout(Duration::from_secs(5))
                .expect("foreground child did not return permission"),
            "returned"
        );
    }
    pub fn close(&mut self) {
        // 只通过所持 Child 的管道请求关闭，不向可能已被复用的缓存 HWND 发消息。
        assert!(
            self.child.try_wait().unwrap().is_none(),
            "UIA child exited before close"
        );
        self.child
            .stdin
            .take()
            .unwrap()
            .write_all(b"close\n")
            .unwrap();
        self.wait_for_exit(true);
    }
    pub fn replace(&mut self) {
        self.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"replace\n")
            .unwrap();
    }
    pub fn terminate(&mut self) {
        // Child 持有进程句柄；不按 PID 名称扫描，也不经 HWND 寻找待终止进程。
        assert!(self.child.try_wait().unwrap().is_none());
        self.child.kill().unwrap();
        self.wait_for_exit(false);
    }
    fn wait_for_exit(&mut self, success: bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert_eq!(
                    status.success(),
                    success,
                    "unexpected fixture exit: {status}"
                );
                self.reader.take().unwrap().join().unwrap();
                return;
            }
            assert!(Instant::now() < deadline, "UIA fixture did not close");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
#[test]
#[ignore = "child entry; only launched by the owned cross-process UIA fixture"]
fn uia_process_fixture_child() {
    assert_eq!(std::env::var("OTTER_NATIVE_UIA_CHILD").as_deref(), Ok("1"));
    let _com = crate::desktop::Apartment::new().unwrap();
    let mode = std::env::var("OTTER_NATIVE_UIA_MODE").unwrap();
    let select = mode == "select";
    let range = mode == "range";
    let foreground = mode == "foreground";
    assert!(select || range || foreground || mode == "default");
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"group","type":"column","children":[{"id":"input","type":"input","props":{"value":"original"},"events":{"change":"changed"}},{"id":"switch","type":"switch","props":{"value":false},"events":{"change":"toggled"}},{"id":"button","type":"button","props":{"label":"Run"},"events":{"click":"run"}}]}"#).unwrap();
    let scene = if select {
        crate::surface_scene::Scene::parse(br#"{"id":"group","type":"column","children":[{"id":"select","type":"select","props":{"value":"a","options":[{"value":"a","label":"First"},{"value":"b","label":"Second"}]},"events":{"change":"choose"}}]}"#).unwrap()
    } else if range {
        crate::surface_scene::Scene::parse(br#"{"id":"slider","type":"slider","props":{"value":50},"events":{"input":"move","change":"change"}}"#).unwrap()
    } else {
        scene
    };
    let window = super::super::super::SurfaceWindow::new(scene, 360, 300).unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.show().unwrap();
    println!("OTTER_UIA_FIXTURE={:x}", window.hwnd.0 as usize);
    std::io::stdout().flush().unwrap();
    let (close_sender, close_receiver) = sync_channel(1);
    let close_reader = std::thread::spawn(move || {
        // 命令总字节数、条数和队列均有界；close 发出后退出读取线程。
        for line in BufReader::new(std::io::stdin()).take(64).lines().take(4) {
            let line = line.unwrap();
            let close = line == "close";
            if close_sender.send(line).is_err() || close {
                break;
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut accepted = false;
    let mut replaced = false;
    let mut range_edges = 0;
    while window.is_alive() {
        assert!(Instant::now() < deadline, "UIA fixture lifetime exceeded");
        if let Ok(command) = close_receiver.try_recv() {
            match command.as_str() {
                "foreground" if foreground => {
                    unsafe {
                        SetWindowPos(
                            window.hwnd,
                            None,
                            -32000,
                            -32000,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        )
                        .unwrap();
                        let _ = SetForegroundWindow(window.hwnd);
                        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(
                            window.hwnd,
                        ));
                        assert_eq!(
                            GetForegroundWindow(),
                            window.hwnd,
                            "owned foreground child activation denied"
                        );
                    }
                    println!("OTTER_UIA_FIXTURE=foreground");
                    std::io::stdout().flush().unwrap();
                }
                "return" if foreground => {
                    unsafe {
                        assert_eq!(GetForegroundWindow(), window.hwnd);
                        let parent = std::env::var("OTTER_NATIVE_UIA_PARENT_PID")
                            .unwrap()
                            .parse::<u32>()
                            .unwrap();
                        assert_ne!(parent, std::process::id());
                        AllowSetForegroundWindow(parent).unwrap();
                    }
                    println!("OTTER_UIA_FIXTURE=returned");
                    std::io::stdout().flush().unwrap();
                }
                "close" => {
                    window.close().unwrap();
                    // close 已使读取线程退出；意外关窗不能 join 尚在读取的线程。
                    close_reader.join().unwrap();
                    break;
                }
                "replace" => {
                    assert!(accepted && !replaced);
                    window.replace(crate::surface_scene::Scene::parse(br#"{"id":"replacement","type":"input","props":{"value":"updated"},"accessibility":{"label":"New tree"}}"#).unwrap()).unwrap();
                    replaced = true;
                }
                _ => panic!("invalid fixture command"),
            }
        }
        unsafe {
            let mut message = MSG::default();
            for _ in 0..64 {
                if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    break;
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if let Some(click) = window.take_click() {
            assert!(!accepted);
            assert_eq!(click.node_id, "button");
            assert!(matches!(
                click.source,
                super::super::super::Activation::Automation
            ));
            assert_eq!(
                click.values.get("input"),
                Some(&serde_json::json!("external"))
            );
            assert!(window.has_pending_edit());
            assert_eq!(
                window
                    .state
                    .host
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .switch_value("switch"),
                Some(true)
            );
            accepted = true;
            println!("OTTER_UIA_FIXTURE=accepted");
            std::io::stdout().flush().unwrap();
        }
        if select && window.has_pending_edit() {
            assert!(!accepted);
            let events = window.prepare_edit(&binding, 4).unwrap();
            assert_eq!(events.len(), 1);
            let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
            assert_eq!(wire["payload"], serde_json::json!({"value":"b"}));
            assert_eq!(wire["class"], "commit");
            window.settle_edits(&events).unwrap();
            accepted = true;
            println!("OTTER_UIA_FIXTURE=accepted");
            std::io::stdout().flush().unwrap();
        }
        if range && window.has_pending_edit() {
            // 客户端依次设置 60、键盘变为 61、取消订阅后设置 70。
            // 从正式 prepare/settle 出口验证六个不可变数值边沿，不能只看 HWND 值。
            let events = window.prepare_edit(&binding, 4).unwrap();
            for event in &events {
                assert!(range_edges < 6, "unexpected duplicate Slider edge");
                let wire: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
                assert_eq!(
                    wire["event"],
                    if range_edges % 2 == 0 {
                        "input"
                    } else {
                        "change"
                    }
                );
                assert_eq!(
                    wire["class"],
                    if range_edges % 2 == 0 {
                        "continuous"
                    } else {
                        "commit"
                    }
                );
                assert_eq!(
                    wire["payload"]["value"],
                    [60.0, 61.0, 70.0][range_edges / 2]
                );
                range_edges += 1;
            }
            window.settle_edits(&events).unwrap();
            if range_edges == 6 && !accepted {
                accepted = true;
                println!("OTTER_UIA_FIXTURE=accepted");
                std::io::stdout().flush().unwrap();
            }
        }
        assert!(window.take_error().is_none());
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        accepted || foreground,
        "fixture closed without receiving its automation action"
    );
}
