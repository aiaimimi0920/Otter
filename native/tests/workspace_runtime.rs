#![cfg(windows)]
//! 真实普通启动→移动→持续检查点→强制结束→重启→删除最后 Unit；只操作自启 PID 的离屏窗口。
use base64::Engine;
use std::{
    fs::OpenOptions,
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use windows::Win32::UI::HiDpi::{
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::{
    core::{w, BOOL},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
const ROOT: &str = "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-workspace-runtime-tests";
#[path = "workspace_runtime/exit.rs"]
mod exit;
#[path = "support/product.rs"]
mod product;
#[path = "workspace_runtime/session_end.rs"]
mod session_end;
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        std::fs::create_dir_all(ROOT).unwrap();
        let path = PathBuf::from(ROOT).join(format!("{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn workspace(&self) -> PathBuf {
        self.0.join("workspace.json")
    }
    fn launch(&self, name: &str) -> OwnedChild {
        let product =
            product::Product::resolve(std::path::Path::new(env!("CARGO_BIN_EXE_otter-native")))
                .unwrap();
        let error = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.0.join(name))
            .unwrap();
        OwnedChild(
            Command::new(product.path())
                .env("OTTER_NATIVE_PROFILE_DIR", &self.0)
                .stdout(Stdio::null())
                .stderr(error)
                .stdin(Stdio::null())
                .creation_flags(0x08000000)
                .spawn()
                .unwrap(),
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("workspace-runtime-failure-evidence: {}", self.0.display());
            return;
        }
        assert_eq!(self.0.parent(), Some(std::path::Path::new(ROOT)));
        for entry in std::fs::read_dir(&self.0).unwrap().flatten() {
            if entry.file_type().unwrap().is_file() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}
struct OwnedChild(Child);
struct Dpi(DPI_AWARENESS_CONTEXT);
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(self.0);
        }
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct Search {
    pid: u32,
    hwnd: Option<HWND>,
}
unsafe extern "system" fn enumerate(hwnd: HWND, parameter: LPARAM) -> BOOL {
    let search = &mut *(parameter.0 as *mut Search);
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == search.pid {
        let mut class = [0u16; 64];
        let size = GetClassNameW(hwnd, &mut class);
        if size > 0 && &class[..size as usize] == w!("Otter.Native.Workspace").as_wide() {
            search.hwnd = Some(hwnd);
        }
    }
    BOOL(1)
}
fn until<T>(child: &mut OwnedChild, stage: &str, mut observe: impl FnMut(u32) -> Option<T>) -> T {
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(value) = observe(child.0.id()) {
            return value;
        }
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "native workspace process ended before {stage} witness"
        );
        assert!(
            Instant::now() < end,
            "native workspace {stage} witness timed out"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn window(child: &mut OwnedChild) -> HWND {
    until(child, "visible-owned-window", |pid| unsafe {
        let mut search = Search { pid, hwnd: None };
        // 空desktop可返回false/ERROR_SUCCESS；callback从不中止，非零错误仍必须失败。
        SetLastError(ERROR_SUCCESS);
        let result = EnumWindows(Some(enumerate), LPARAM(&mut search as *mut Search as isize));
        enumerated_window(result, search.hwnd)
            .unwrap()
            .filter(|hwnd| IsWindowVisible(*hwnd).as_bool())
    })
}
fn enumerated_window(
    result: windows::core::Result<()>,
    hwnd: Option<HWND>,
) -> windows::core::Result<Option<HWND>> {
    match result {
        Err(error) if error.code().0 == 0 && hwnd.is_none() => Ok(None),
        result => result.map(|()| hwnd),
    }
}
#[test]
fn empty_desktop_enumeration_waits_but_real_errors_and_partial_results_fail() {
    use windows::core::{Error, HRESULT};
    let zero = || Error::from_hresult(HRESULT(0));
    assert_eq!(enumerated_window(Err(zero()), None).unwrap(), None);
    let owned = HWND(std::ptr::dangling_mut());
    assert_eq!(enumerated_window(Ok(()), Some(owned)).unwrap(), Some(owned));
    assert!(enumerated_window(Err(zero()), Some(owned)).is_err());
    let denied = Error::from_hresult(HRESULT::from_win32(ERROR_ACCESS_DENIED.0));
    assert_eq!(
        enumerated_window(Err(denied), None).unwrap_err().code(),
        HRESULT::from_win32(ERROR_ACCESS_DENIED.0)
    );
}
#[test]
fn ordinary_product_startup_recovers_persists_restarts_and_does_not_resurrect_deleted_last_unit() {
    // SetWindowPos/GetWindowRect 是调用线程 DPI 相关的跨进程 API；产品持久化使用物理坐标。
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    let fixture = Fixture::new();
    let image = base64::engine::general_purpose::STANDARD
        .encode(include_bytes!("../../src-tauri/icons/32x32.png"));
    let source = serde_json::json!({
        "documentSchemaVersion":1,"documentRevision":7,
        "otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},
        "stickers":[{"id":"workspace-runtime","x":-16000,"y":-16000,"w":32,"h":32,"src":format!("data:image/png;base64,{image}")}],
        "links":[],"groups":[],"extension":{"preserved":true}
    });
    std::fs::write(fixture.workspace(), serde_json::to_vec(&source).unwrap()).unwrap();
    let mut first = fixture.launch("first-stderr.log");
    let hwnd = window(&mut first);
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            -15998,
            -15997,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    let persisted = until(&mut first, "checkpoint-moved-window", |_| {
        let bytes = std::fs::read(fixture.workspace()).ok()?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        (value["documentRevision"].as_u64()? >= 8
            && value["stickers"][0]["x"].as_f64() == Some(-15998.0)
            && value["stickers"][0]["y"].as_f64() == Some(-15997.0))
        .then_some(bytes)
    });
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&persisted).unwrap()["extension"]["preserved"],
        true
    );
    first.0.kill().unwrap();
    first.0.wait().unwrap(); // 无 Drop/正常退出 flush 的真实进程中断。
    drop(first);
    let mut second = fixture.launch("second-stderr.log");
    let hwnd = window(&mut second);
    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).unwrap();
        assert_eq!((rect.left, rect.top), (-15998, -15997));
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        assert_eq!(
            pid,
            second.0.id(),
            "refuse to close any other process window"
        );
        PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap();
    }
    let end = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = second.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < end,
            "native empty workspace exit timed out"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "{}",
        std::fs::read_to_string(fixture.0.join("second-stderr.log")).unwrap()
    );
    let saved = std::fs::read(fixture.workspace()).unwrap();
    assert!(otter_native::session_document::decode(&saved)
        .unwrap()
        .workspace
        .units()
        .is_empty());
    drop(second);
}
