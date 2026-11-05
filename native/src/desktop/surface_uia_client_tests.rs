//! 独立 MTA 客户线程经系统 UIA 读取真实隐藏 HWND，UI 线程泵送 COM 消息。
use super::*;
use std::time::{Duration, Instant};
use windows::Win32::UI::WindowsAndMessaging::*;
#[path = "surface_uia_event_tests.rs"]
mod events;
#[path = "surface_uia_focus_event_tests.rs"]
mod focus_events;
#[path = "surface_uia_image_client_tests.rs"]
mod images;
#[path = "surface_uia_edit_client_tests.rs"]
mod native_edit;
#[path = "surface_uia_select_client_tests.rs"]
pub(super) mod native_select;
#[path = "surface_uia_range_client_tests.rs"]
mod range;
#[path = "surface_uia_range_event_tests.rs"]
pub(super) mod range_events;
#[path = "surface_uia_range_readonly_tests.rs"]
mod range_readonly;
#[path = "surface_uia_structure_event_tests.rs"]
mod structure_events;
#[path = "surface_uia_toggle_client_tests.rs"]
mod toggle;
#[path = "surface_uia_toggle_event_tests.rs"]
mod toggle_events;

struct Apartment;
// 测试自有 HWND 的有界消息证据；无裸 Rust 指针，不吞消息、不改变输入行为。
struct InputTrace(HWND);
impl InputTrace {
    fn new(hwnd: HWND) -> Self {
        // 同步采证也会扰动时序；普通回归默认完全不安装，诊断必须显式开启。
        if std::env::var_os("OTTER_UIA_INPUT_TRACE").as_deref() != Some(std::ffi::OsStr::new("1")) {
            return Self(HWND::default());
        }
        assert!(unsafe {
            windows::Win32::UI::Shell::SetWindowSubclass(hwnd, Some(input_trace), 0x4f545549, 32)
        }
        .as_bool());
        Self(hwnd)
    }
}
impl Drop for InputTrace {
    fn drop(&mut self) {
        if self.0.is_invalid() {
            return;
        }
        unsafe {
            let _ = windows::Win32::UI::Shell::RemoveWindowSubclass(
                self.0,
                Some(input_trace),
                0x4f545549,
            );
        }
    }
}
unsafe extern "system" fn input_trace(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    remaining: usize,
) -> LRESULT {
    if remaining != 0
        && matches!(
            message,
            WM_CHAR
                | WM_KEYDOWN
                | WM_SETFOCUS
                | WM_KILLFOCUS
                | WM_SETTEXT
                | WM_COMMAND
                | CB_SHOWDROPDOWN
        )
    {
        let _ = windows::Win32::UI::Shell::SetWindowSubclass(
            hwnd,
            Some(input_trace),
            id,
            remaining - 1,
        );
        // 同一消息现场绑定前台 HWND 的进程，避免事后复用句柄时误认夺焦方。
        let foreground = GetForegroundWindow();
        let mut foreground_pid = 0;
        let foreground_thread = GetWindowThreadProcessId(foreground, Some(&mut foreground_pid));
        eprintln!(
            "UIA input trace: hwnd={hwnd:?}, message={message:#x}, wp={:#x}, lp={:#x}, extra={:?}, foreground={foreground:?}, foreground_pid={foreground_pid}, foreground_thread={foreground_thread}, own_pid={}",
            wp.0, lp.0, GetMessageExtraInfo(), std::process::id()
        );
    }
    windows::Win32::UI::Shell::DefSubclassProc(hwnd, message, wp, lp)
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
fn read(hwnd: usize) -> Result<(String, String, i32)> {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
    let _apartment = Apartment;
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let element = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        Ok((
            element.CurrentName()?.to_string(),
            element.CurrentFrameworkId()?.to_string(),
            element.CurrentControlType()?.0,
        ))
    }
}

fn query<T: Send + 'static>(hwnd: usize, read: fn(usize) -> Result<T>) -> T {
    query_with(hwnd, read, || {})
}
fn query_with<T: Send + 'static>(
    hwnd: usize,
    read: impl FnOnce(usize) -> Result<T> + Send + 'static,
    mut tick: impl FnMut(),
) -> T {
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        // read 返回前释放 element/client，再退出 MTA；发送后 join 不需继续泵 COM。
        let result = read(hwnd).map_err(|error| (error.code(), error.message()));
        let _ = sender.send(result);
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    let result = loop {
        tick();
        match receiver.try_recv() {
            Ok(result) => break result,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => panic!("UIA client disconnected"),
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        assert!(Instant::now() < deadline, "UIA client timed out");
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
        std::thread::sleep(Duration::from_millis(1));
    };
    worker.join().unwrap();
    result.unwrap()
}

#[test]
fn uia_root_system_client_reads_native_provider_on_sta_window() {
    let _com = super::super::super::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(
        br#"{"id":"root","type":"text","props":{"text":"Text"}}"#,
    )
    .unwrap();
    let window = super::super::SurfaceWindow::new(scene, 200, 140).unwrap();
    assert_eq!(
        query(window.hwnd.0 as usize, read),
        (
            "Otter Surface".into(),
            "Otter.Native".into(),
            UIA_PaneControlTypeId.0
        )
    );
    assert!(window.state.uia.borrow().is_some());
    assert!(window.take_error().is_none());
    window.close().unwrap();
}

fn read_tree(hwnd: usize) -> Result<Vec<String>> {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
    let _apartment = Apartment;
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let window = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let condition = client.CreatePropertyCondition(
            UIA_AutomationIdPropertyId,
            &windows::Win32::System::Variant::VARIANT::from("tree"),
        )?;
        let tree = window.FindFirst(TreeScope_Descendants, &condition)?;
        let walker = client.RawViewWalker()?;
        let first = walker.GetFirstChildElement(&tree)?;
        let rectangle = first.CurrentBoundingRectangle()?;
        assert!(rectangle.right > rectangle.left && rectangle.bottom > rectangle.top);
        assert!(first.CurrentIsOffscreen()?.as_bool());
        let last = walker.GetNextSiblingElement(&first)?;
        Ok(vec![
            tree.CurrentAutomationId()?.to_string(),
            first.CurrentName()?.to_string(),
            last.CurrentName()?.to_string(),
            last.CurrentAutomationId()?.to_string(),
        ])
    }
}

fn focus_button(hwnd: usize) -> Result<(bool, bool)> {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
    let _apartment = Apartment;
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let window = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let condition = client.CreatePropertyCondition(
            UIA_AutomationIdPropertyId,
            &windows::Win32::System::Variant::VARIANT::from("button"),
        )?;
        let button = window.FindFirst(TreeScope_Descendants, &condition)?;
        let focusable = button.CurrentIsKeyboardFocusable()?.as_bool();
        button.SetFocus()?;
        Ok((focusable, button.CurrentHasKeyboardFocus()?.as_bool()))
    }
}

fn invoke_button(hwnd: usize) -> Result<()> {
    invoke_button_with_focus(hwnd, true)
}
fn invoke_button_without_focus(hwnd: usize) -> Result<()> {
    invoke_button_with_focus(hwnd, false)
}
fn invoke_button_with_focus(hwnd: usize, auto_focus: bool) -> Result<()> {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
    let _apartment = Apartment;
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        client.SetAutoSetFocus(auto_focus)?;
        let window = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let condition = client.CreatePropertyCondition(
            UIA_AutomationIdPropertyId,
            &windows::Win32::System::Variant::VARIANT::from("button"),
        )?;
        let button = window.FindFirst(TreeScope_Descendants, &condition)?;
        let invoke: IUIAutomationInvokePattern = button.GetCurrentPatternAs(UIA_InvokePatternId)?;
        invoke.Invoke()
    }
}

#[test]
fn uia_invoke_system_client_queues_button_action() {
    let _com = super::super::super::Apartment::new().unwrap();
    for auto_focus in [false, true] {
        let scene = crate::surface_scene::Scene::parse(
            br#"{"id":"button","type":"button","props":{"label":"Run"},"events":{"click":"run"}}"#,
        )
        .unwrap();
        let window = super::super::SurfaceWindow::new(scene, 200, 140).unwrap();
        window.set_input_policy(true, true).unwrap();
        unsafe {
            let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        }
        query(
            window.hwnd.0 as usize,
            if auto_focus {
                invoke_button
            } else {
                invoke_button_without_focus
            },
        );
        let click = window.take_click().unwrap();
        assert_eq!(click.node_id, "button");
        assert!(matches!(click.source, super::super::Activation::Automation));
        // AutoSetFocus 是客户端策略，不保证消息泵结束后窗口仍持焦点。
        // 显式 SetFocus 的合同由独立焦点测试覆盖；这里只断言禁用时不抢焦点。
        if !auto_focus {
            assert!(window.state.keyboard.borrow().focused().is_none());
        }
        assert!(window.take_error().is_none());
    }
}

#[test]
fn uia_focus_system_client_sets_declarative_button_without_click() {
    let _com = super::super::super::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(
        br#"{"id":"button","type":"button","props":{"label":"Run"},"events":{"click":"run"}}"#,
    )
    .unwrap();
    let window = super::super::SurfaceWindow::new(scene, 200, 140).unwrap();
    window.set_input_policy(true, false).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
    }
    assert_eq!(query(window.hwnd.0 as usize, focus_button), (true, true));
    assert_eq!(window.state.keyboard.borrow().focused(), Some("button"));
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}

#[test]
fn uia_fragment_system_client_walks_current_declarative_tree() {
    let _com = super::super::super::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"tree","type":"column","children":[{"id":"button","type":"button","props":{"label":"Run"}},{"id":"input","type":"input"},{"id":"text","type":"text","props":{"text":"Ready"}}]}"#).unwrap();
    let window = super::super::SurfaceWindow::new(scene, 200, 140).unwrap();
    assert_eq!(
        query(window.hwnd.0 as usize, read_tree),
        ["tree", "Run", "Ready", "text"]
    );
    let replacement = crate::surface_scene::Scene::parse(br#"{"id":"tree","type":"column","children":[{"id":"button","type":"button","props":{"label":"Again"}},{"id":"text","type":"text","props":{"text":"Updated"}}]}"#).unwrap();
    window.replace(replacement).unwrap();
    assert_eq!(
        query(window.hwnd.0 as usize, read_tree),
        ["tree", "Again", "Updated", "text"]
    );
    window.close().unwrap();
}
