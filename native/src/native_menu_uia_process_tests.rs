//! 独立客户端进程验证 UIA 跨进程封送；仅传递测试 HWND，不接触用户文档。
use super::*;
use std::process::{Child, Command, Stdio};

const CHILD_TEST: &str = "native_menu::cascade::tests::uia_client::process::uia_client_child";
const HWND_ENV: &str = "OTTER_TEST_UIA_HWND";
const PID_ENV: &str = "OTTER_TEST_UIA_PID";
const EVENT_TEST: &str = "native_menu::cascade::tests::uia_client::process::uia_event_client_child";
const MODE_ENV: &str = "OTTER_TEST_UIA_MODE";

// panic/超时也必须回收自己启动的客户端，不结束任何其他进程。
struct Client(Child);
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn separate_process_expands_scrolls_invokes_and_observes_revocation() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let owner = Owner::new();
    let mut parent = item(1);
    let mut children: Vec<_> = (10..40).map(item).collect();
    children[29].checked = true;
    children[29].radio = true;
    parent.children = Some(children);
    let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 30, y: 30 }).unwrap();
    run_process(&mut menu, CHILD_TEST, 0);
    assert!(menu.closed());
    assert_eq!(menu.take_command(), Some(39));
    assert!(menu.take_command().is_none());
}

fn run_process(menu: &mut Cascade, test: &str, mode: u8) {
    let hwnd = menu.pages[0].panel.handle().unwrap().0 as usize;
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            test,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(HWND_ENV, hwnd.to_string())
        .env(PID_ENV, std::process::id().to_string())
        .env(MODE_ENV, mode.to_string())
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    assert_ne!(child.id(), std::process::id());
    let mut child = Client(child);
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        menu.poll().unwrap();
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "UIA child failed: {status}");
            break;
        }
        assert!(Instant::now() < deadline, "UIA child exceeded deadline");
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
    }
}

#[test]
fn separate_process_receives_single_invoked_event_for_all_selection_paths() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let owner = Owner::new();
    // 两种订阅范围各覆盖 UIA、键盘消息和鼠标消息；每例独立进程。
    for mode in 0..6 {
        let mut command = item(42);
        command.label = invoked::NAME.into();
        let mut menu = Cascade::open(owner.0, vec![command], POINT { x: 30, y: 30 }).unwrap();
        run_process(&mut menu, EVENT_TEST, mode);
        assert!(menu.closed());
        assert_eq!(menu.take_command(), Some(42));
        assert!(menu.take_command().is_none());
    }
}

#[test]
#[ignore = "Only launched by the bounded parent with a private test HWND"]
fn uia_event_client_child() {
    let hwnd: usize = std::env::var(HWND_ENV).unwrap().parse().unwrap();
    let pid: u32 = std::env::var(PID_ENV).unwrap().parse().unwrap();
    let mode: u8 = std::env::var(MODE_ENV).unwrap().parse().unwrap();
    assert_ne!(pid, std::process::id());
    assert!(mode < 6);
    let mut actual_pid = 0;
    unsafe {
        GetWindowThreadProcessId(HWND(hwnd as *mut _), Some(&mut actual_pid));
    }
    assert_eq!(actual_pid, pid);
    // 复用同进程回归的有界订阅/缓存/去重/注销后 Drop 断言，不另写弱化合同。
    let scope = if mode >= 3 {
        invoked::Scope::Desktop
    } else {
        invoked::Scope::Source
    };
    invoked::receive_scoped(hwnd, mode % 3, scope).unwrap();
}

#[test]
#[ignore = "Only launched by the bounded parent with a private test HWND"]
fn uia_client_child() {
    let hwnd: usize = std::env::var(HWND_ENV).unwrap().parse().unwrap();
    let pid: u32 = std::env::var(PID_ENV).unwrap().parse().unwrap();
    assert_ne!(pid, std::process::id());
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
    }
    let _com = Apartment;
    unsafe {
        run_client(hwnd, pid).unwrap();
    }
}

unsafe fn run_client(hwnd: usize, pid: u32) -> Result<()> {
    let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
    client.SetConnectionTimeout(3000)?;
    client.SetTransactionTimeout(3000)?;
    let root = client.ElementFromHandle(HWND(hwnd as *mut _))?;
    assert_eq!(root.CurrentProcessId()?, pid as i32);
    assert_eq!(root.CurrentName()?.to_string(), "Otter 菜单");
    let walker = client.RawViewWalker()?;
    let parent = walker.GetFirstChildElement(&root)?;
    let expand: IUIAutomationExpandCollapsePattern =
        parent.GetCurrentPatternAs(UIA_ExpandCollapsePatternId)?;
    expand.Expand()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while expand.CurrentExpandCollapseState()? != ExpandCollapseState_Expanded {
        assert!(Instant::now() < deadline, "UIA expansion timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
    let submenu = walker.GetFirstChildElement(&parent)?;
    assert_eq!(submenu.CurrentProcessId()?, pid as i32);
    let old = walker.GetFirstChildElement(&submenu)?;
    let old_invoke: IUIAutomationInvokePattern = old.GetCurrentPatternAs(UIA_InvokePatternId)?;
    let scroll: IUIAutomationScrollPattern = submenu.GetCurrentPatternAs(UIA_ScrollPatternId)?;
    assert!(scroll.CurrentVerticallyScrollable()?.as_bool());
    scroll.SetScrollPercent(UIA_ScrollPatternNoScroll, 100.0)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while scroll.CurrentVerticalScrollPercent()? != 100.0 {
        assert!(Instant::now() < deadline, "UIA scroll timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        old_invoke.Invoke().unwrap_err().code().0,
        UIA_E_ELEMENTNOTAVAILABLE as i32
    );
    let leaf = walker.GetLastChildElement(&submenu)?;
    assert_eq!(leaf.CurrentName()?.to_string(), "命令 39");
    assert_eq!(leaf.CurrentItemStatus()?.to_string(), "已选中");
    let invoke: IUIAutomationInvokePattern = leaf.GetCurrentPatternAs(UIA_InvokePatternId)?;
    invoke.Invoke()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match leaf.CurrentName() {
            Err(error) => {
                assert_eq!(error.code().0, UIA_E_ELEMENTNOTAVAILABLE as i32);
                break;
            }
            Ok(_) => {
                assert!(
                    Instant::now() < deadline,
                    "UIA leaf remained available after invocation"
                );
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    Ok(())
}
