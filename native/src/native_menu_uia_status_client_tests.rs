//! 经 Windows UIA MTA 客户端读取真实菜单标记，不触发菜单命令。
use super::*;

fn read_statuses(hwnd: usize) -> Result<Vec<String>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _com = Apartment;
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let root = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        assert_eq!(root.CurrentItemStatus()?.to_string(), "");
        let walker = client.RawViewWalker()?;
        let mut row = walker.GetFirstChildElement(&root)?;
        let mut statuses = Vec::with_capacity(4);
        for index in 0..4 {
            statuses.push(row.CurrentItemStatus()?.to_string());
            if index < 3 {
                row = walker.GetNextSiblingElement(&row)?;
            }
        }
        Ok(statuses)
    }
}

#[test]
fn windows_uia_client_reads_checked_and_radio_status_without_executing() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let owner = Owner::new();
    let mut items = vec![item(1), item(2), item(3), item(4)];
    items[1].checked = true;
    items[2].radio = true;
    items[3].radio = true;
    items[3].checked = true;
    items[3].enabled = false;
    let mut menu = Cascade::open(owner.0, items, POINT { x: 30, y: 30 }).unwrap();
    let hwnd = menu.pages[0].panel.handle().unwrap().0 as usize;
    assert_eq!(
        query(hwnd, read_statuses, || {}),
        ["", "已勾选", "未选中", "已选中"]
    );
    menu.poll().unwrap();
    assert!(!menu.closed());
    assert!(menu.take_command().is_none());
}
