//! 系统 UIA 沿父行进入弹出子菜单，验证返回路径与稳定身份，不直接查找子 HWND。
use super::*;
fn walk_and_invoke(hwnd: usize) -> Result<()> {
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
        let parent = walker.GetFirstChildElement(&root)?;
        let expand: IUIAutomationExpandCollapsePattern =
            parent.GetCurrentPatternAs(UIA_ExpandCollapsePatternId)?;
        expand.Expand()?;
        let deadline = Instant::now() + Duration::from_secs(3);
        while expand.CurrentExpandCollapseState()? != ExpandCollapseState_Expanded {
            if Instant::now() >= deadline {
                return Err(Error::new(E_FAIL, "Submenu tree did not settle"));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let popup = walker.GetFirstChildElement(&parent)?;
        assert_eq!(popup.CurrentControlType()?, UIA_MenuControlTypeId);
        let first = walker.GetFirstChildElement(&popup)?;
        let last = walker.GetLastChildElement(&popup)?;
        assert_eq!(first.CurrentName()?.to_string(), "命令 42");
        assert_eq!(last.CurrentName()?.to_string(), "命令 43");
        let next = walker.GetNextSiblingElement(&first)?;
        assert!(client.CompareElements(&next, &last)?.as_bool());
        let back_popup = walker.GetParentElement(&first)?;
        assert!(client.CompareElements(&popup, &back_popup)?.as_bool());
        let back_parent = walker.GetParentElement(&popup)?;
        assert!(client.CompareElements(&parent, &back_parent)?.as_bool());
        let back_root = walker.GetParentElement(&back_parent)?;
        assert!(client.CompareElements(&root, &back_root)?.as_bool());
        let invoke: IUIAutomationInvokePattern = first.GetCurrentPatternAs(UIA_InvokePatternId)?;
        invoke.Invoke()?;
    }
    Ok(())
}
#[test]
fn windows_uia_walks_across_popup_hwnd_and_returns_to_same_logical_parent() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let owner = Owner::new();
    let mut parent = item(1);
    parent.children = Some(vec![item(42), item(43)]);
    let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 30, y: 30 }).unwrap();
    let hwnd = menu.pages[0].panel.handle().unwrap().0 as usize;
    query(hwnd, walk_and_invoke, || menu.poll().unwrap());
    menu.poll().unwrap();
    assert_eq!(menu.take_command(), Some(42));
    assert!(menu.closed());
}

fn focus_child_then_parent(hwnd: usize) -> Result<()> {
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
        let parent = walker.GetFirstChildElement(&root)?;
        let expand: IUIAutomationExpandCollapsePattern =
            parent.GetCurrentPatternAs(UIA_ExpandCollapsePatternId)?;
        expand.Expand()?;
        let deadline = Instant::now() + Duration::from_secs(3);
        while expand.CurrentExpandCollapseState()? != ExpandCollapseState_Expanded {
            if Instant::now() >= deadline {
                return Err(Error::new(E_FAIL, "Submenu did not expand"));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let popup = walker.GetFirstChildElement(&parent)?;
        let child = walker.GetFirstChildElement(&popup)?;
        child.SetFocus()?;
        wait_focus(&child)?;
        assert!(!parent.CurrentHasKeyboardFocus()?.as_bool());
        parent.SetFocus()?;
        wait_focus(&parent)?;
        assert_eq!(
            expand.CurrentExpandCollapseState()?,
            ExpandCollapseState_Collapsed
        );
        assert!(child.CurrentName().is_err());
    }
    Ok(())
}
unsafe fn wait_focus(element: &IUIAutomationElement) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !element.CurrentHasKeyboardFocus()?.as_bool() {
        if Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Menu focus did not settle"));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}
#[test]
fn windows_uia_focus_returns_from_child_to_highlighted_parent_without_duplicate_focus() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let owner = Owner::new();
    let mut parent = item(1);
    parent.children = Some(vec![item(42)]);
    let mut menu = Cascade::open(owner.0, vec![parent], POINT { x: 30, y: 30 }).unwrap();
    let hwnd = menu.pages[0].panel.handle().unwrap().0 as usize;
    query(hwnd, focus_child_then_parent, || menu.poll().unwrap());
    assert_eq!(menu.pages.len(), 1);
    assert_eq!(menu.pages[0].focus, Some(0));
    assert!(menu.take_command().is_none());
}
