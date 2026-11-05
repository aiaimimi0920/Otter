//! 独立 MTA 系统 UIA 客户端发现菜单并 Invoke；STA 泵消息后才消费业务命令。
#[path = "native_menu_uia_event_client_tests.rs"]
mod events;
#[path = "native_menu_uia_invoked_client_tests.rs"]
mod invoked;
#[path = "native_menu_uia_process_tests.rs"]
mod process;
#[path = "native_menu_uia_status_client_tests.rs"]
mod status;
#[path = "native_menu_uia_tree_client_tests.rs"]
mod tree;
use super::*;
use std::time::{Duration, Instant};
use windows::Win32::{System::Com::*, UI::Accessibility::*};
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
fn read_and_invoke(hwnd: usize) -> Result<(String, String, i32)> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _com = Apartment;
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let root = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let row = client.RawViewWalker()?.GetFirstChildElement(&root)?;
        let result = (
            root.CurrentName()?.to_string(),
            row.CurrentName()?.to_string(),
            row.CurrentControlType()?.0,
        );
        let invoke: IUIAutomationInvokePattern = row.GetCurrentPatternAs(UIA_InvokePatternId)?;
        invoke.Invoke()?;
        Ok(result)
    }
}
fn query<T: Send + 'static>(
    hwnd: usize,
    read: fn(usize) -> Result<T>,
    mut tick: impl FnMut(),
) -> T {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let result = read(hwnd).map_err(|error| (error.code(), error.message()));
        let _ = sender.send(result);
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    let result = loop {
        tick();
        match receiver.try_recv() {
            Ok(result) => break result,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => panic!("UIA worker disconnected"),
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        assert!(Instant::now() < deadline, "UIA menu discovery timed out");
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
fn windows_uia_client_discovers_real_menu_and_queues_leaf_command() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let owner = Owner::new();
    let mut menu = Cascade::open(owner.0, vec![item(42)], POINT { x: 30, y: 30 }).unwrap();
    let hwnd = menu.pages[0].panel.handle().unwrap().0 as usize;
    assert_eq!(
        query(hwnd, read_and_invoke, || {}),
        (
            "Otter 菜单".into(),
            "命令 42".into(),
            UIA_MenuItemControlTypeId.0
        )
    );
    assert!(!menu.closed());
    assert!(menu.take_command().is_none());
    menu.poll().unwrap();
    assert!(menu.closed());
    assert_eq!(menu.take_command(), Some(42));
}

fn expand_and_collapse(hwnd: usize) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _com = Apartment;
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let root = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let row = client.RawViewWalker()?.GetFirstChildElement(&root)?;
        let pattern: IUIAutomationExpandCollapsePattern =
            row.GetCurrentPatternAs(UIA_ExpandCollapsePatternId)?;
        assert_eq!(
            pattern.CurrentExpandCollapseState()?,
            ExpandCollapseState_Collapsed
        );
        pattern.Expand()?;
        for expected in [ExpandCollapseState_Expanded, ExpandCollapseState_Collapsed] {
            let deadline = Instant::now() + Duration::from_secs(3);
            while pattern.CurrentExpandCollapseState()? != expected {
                if Instant::now() >= deadline {
                    return Err(Error::new(E_FAIL, "Menu expansion did not settle"));
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            if expected == ExpandCollapseState_Expanded {
                pattern.Collapse()?;
            }
        }
    }
    Ok(())
}
#[test]
fn windows_uia_submenu_pattern_tracks_actual_controller_expansion_and_collapse() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let owner = Owner::new();
    let mut parent = item(1);
    parent.children = Some(vec![item(42)]);
    let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 30, y: 30 }).unwrap();
    let hwnd = menu.pages[0].panel.handle().unwrap().0 as usize;
    let mut child = None;
    query(hwnd, expand_and_collapse, || {
        menu.poll().unwrap();
        if menu.pages.len() == 2 && child.is_none() {
            child = menu.pages[1].panel.accessibility_nodes().into_iter().next();
        }
    });
    assert_eq!(menu.pages.len(), 1);
    assert!(menu.take_command().is_none());
    assert!(child
        .expect("actual submenu was never created")
        .read()
        .is_none());
}

fn scroll_and_invoke(hwnd: usize) -> Result<String> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _com = Apartment;
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let root = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let walker = client.RawViewWalker()?;
        let old = walker.GetFirstChildElement(&root)?;
        let old_invoke: IUIAutomationInvokePattern =
            old.GetCurrentPatternAs(UIA_InvokePatternId)?;
        let scroll: IUIAutomationScrollPattern = root.GetCurrentPatternAs(UIA_ScrollPatternId)?;
        assert!(scroll.CurrentVerticallyScrollable()?.as_bool());
        scroll.SetScrollPercent(UIA_ScrollPatternNoScroll, 100.0)?;
        let deadline = Instant::now() + Duration::from_secs(3);
        while scroll.CurrentVerticalScrollPercent()? < 100.0 {
            if Instant::now() >= deadline {
                return Err(Error::new(E_FAIL, "Menu scroll did not settle"));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(old_invoke.Invoke().is_err());
        let current = walker.GetFirstChildElement(&root)?;
        let name = current.CurrentName()?.to_string();
        let invoke: IUIAutomationInvokePattern =
            current.GetCurrentPatternAs(UIA_InvokePatternId)?;
        invoke.Invoke()?;
        Ok(name)
    }
}
#[test]
fn windows_uia_scroll_reaches_hidden_rows_and_revokes_old_visible_identity() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let owner = Owner::new();
    let mut menu =
        Cascade::open(owner.0, (0..40).map(item).collect(), POINT { x: 30, y: 30 }).unwrap();
    let expected = 40 - menu.pages[0].visible;
    let hwnd = menu.pages[0].panel.handle().unwrap().0 as usize;
    let name = query(hwnd, scroll_and_invoke, || menu.poll().unwrap());
    menu.poll().unwrap();
    assert_eq!(name, format!("命令 {expected}"));
    assert_eq!(menu.take_command(), Some(expected));
    assert!(menu.closed());
}
