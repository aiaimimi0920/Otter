//! 显式 CLR/WinForms 门禁；只启动指定 fixture，并验证 HWND 属于持有的子进程。
use super::*;
use crate::live_input::{Button, Controller};
use std::{
    path::Path,
    process::{Child, Command},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        // 只回收本测试直接创建的 fixture，不按名称搜索或终止用户应用。
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn document(path: &Path, condition: impl Fn(&serde_json::Value) -> bool) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(value) = std::fs::read(path)
            .ok()
            .filter(|bytes| bytes.len() <= 16384)
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        {
            if condition(&value) {
                return value;
            }
        }
        assert!(
            Instant::now() < deadline,
            "fixture document timeout: {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[ignore = "interactive desktop; requires OTTER_NATIVE_WINFORMS_FIXTURE compiled from repository C# fixture"]
fn actual_managed_winforms_click_drag_cancel_and_restore() {
    let executable =
        std::env::var_os("OTTER_NATIVE_WINFORMS_FIXTURE").expect("explicit fixture executable");
    let base = Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi");
    let directory = base.join(format!(
        "otter-winforms-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let ready_path = directory.join("ready.json");
    let state_path = directory.join("state.json");
    let process = Process(
        Command::new(executable)
            .arg(&ready_path)
            .arg("0")
            .arg(&state_path)
            .env("HOOK_LIVE_FIXTURE_BACKGROUND", "1")
            .spawn()
            .unwrap(),
    );
    let ready = document(&ready_path, |_| true);
    let hwnd = usize::from_str_radix(ready["hwnd"].as_str().unwrap().trim_start_matches("0x"), 16)
        .unwrap();
    let hwnd = HWND(hwnd as *mut _);
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    assert_eq!(
        pid,
        process.0.id(),
        "fixture HWND must belong to owned child"
    );
    let target = Target::bind(hwnd).unwrap();
    let original = super::super::snapshot::Snapshot::read(&target).unwrap();
    unsafe {
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            original.extended_style | (WS_EX_LAYERED | WS_EX_TRANSPARENT).0 as isize,
        );
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 1, LWA_ALPHA).unwrap();
    }
    let point = [
        ready["targets"]["action"][0].as_f64().unwrap(),
        ready["targets"]["action"][1].as_f64().unwrap(),
    ];
    let drag = [
        ready["targets"]["dragEnd"][0].as_f64().unwrap(),
        ready["targets"]["dragEnd"][1].as_f64().unwrap(),
    ];
    let edge = |down| Event::Button {
        point,
        button: Button::Left,
        down,
        double: false,
    };
    let mut controller = Controller::new(Backend::new(target.clone(), None).unwrap());
    controller.enable().unwrap();
    controller.dispatch(1, edge(true)).unwrap();
    controller.dispatch(2, edge(false)).unwrap();
    let clicked = document(&state_path, |state| state["clicks"] == 1);
    assert_eq!(clicked["pressedMouseButtons"], 0);
    controller.dispatch(3, edge(true)).unwrap();
    controller.dispatch(4, Event::Move { point: drag }).unwrap();
    controller.dispatch(5, Event::Move { point }).unwrap();
    controller.dispatch(6, edge(false)).unwrap();
    let dragged = document(&state_path, |state| {
        state["dragEdges"].as_u64().unwrap_or(0) >= 6
    });
    assert_eq!(dragged["clicks"], 1, "drag-return must not become Click");
    controller.dispatch(7, edge(true)).unwrap();
    assert_eq!(controller.cancel().released, 1);
    let cancelled = document(&state_path, |state| {
        state["dragEdges"].as_u64().unwrap_or(0) >= 8
    });
    assert_eq!(cancelled["clicks"], 1, "cleanup must not become Click");
    assert_eq!(cancelled["pressedMouseButtons"], 0);
    original.restore().unwrap();
    assert_eq!(
        super::super::snapshot::Snapshot::read(&target)
            .unwrap()
            .extended_style,
        original.extended_style
    );
    eprintln!(
        "native-winforms: managed-click=1 drag-click=0 cleanup-click=0 restored=true artifacts={}",
        directory.display()
    );
}
