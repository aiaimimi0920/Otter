//! 托盘根菜单下角锚定与边缘避让；验证真实 HWND，不移动用户鼠标。
use super::*;

fn work_area() -> RECT {
    let mut monitor = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        assert!(GetMonitorInfoW(
            MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTONEAREST),
            &mut monitor,
        )
        .as_bool());
    }
    monitor.rcWork
}
fn bounds(menu: &Cascade, level: usize) -> RECT {
    let page = &menu.pages[level];
    let mut actual = RECT::default();
    unsafe { GetWindowRect(page.panel.handle().unwrap(), &mut actual).unwrap() };
    assert_eq!(actual, page.bounds);
    actual
}
fn assert_inside(rect: RECT, work: RECT) {
    assert!(rect.left >= work.left && rect.right <= work.right);
    assert!(rect.top >= work.top && rect.bottom <= work.bottom);
    assert!(rect.right > rect.left && rect.bottom > rect.top);
}
fn tray(owner: HWND, point: POINT) -> Cascade {
    Cascade::open_with_tray_mode(owner, tree(), point, true).unwrap()
}

#[test]
fn tray_root_anchors_bottom_left_or_right_to_the_requested_point() {
    let owner = Owner::new();
    let work = work_area();
    let y = work.top + (work.bottom - work.top) / 2;
    for (x, right) in [(work.left + 8, false), (work.right - 8, true)] {
        let point = POINT { x, y };
        let menu = tray(owner.0, point);
        let rect = bounds(&menu, 0);
        assert_inside(rect, work);
        assert_eq!(rect.bottom, y);
        assert_eq!(if right { rect.right } else { rect.left }, x);
        assert!(!unsafe { IsWindowVisible(owner.0).as_bool() });
    }
}

#[test]
fn tray_position_yields_to_work_area_at_edges_without_changing_size() {
    let owner = Owner::new();
    let work = work_area();
    let baseline = tray(
        owner.0,
        POINT {
            x: work.left + 8,
            y: work.top + (work.bottom - work.top) / 2,
        },
    );
    let expected = bounds(&baseline, 0);
    drop(baseline);
    // 顶部空间不足及任务栏区域不强求不可能的下角贴合，保证菜单完整可见。
    for point in [
        POINT {
            x: work.left + 8,
            y: work.top + 1,
        },
        POINT {
            x: work.right - 8,
            y: work.top + 1,
        },
        POINT {
            x: work.right - 8,
            y: work.bottom + 8,
        },
    ] {
        let menu = tray(owner.0, point);
        let rect = bounds(&menu, 0);
        assert_inside(rect, work);
        assert_eq!(rect.right - rect.left, expected.right - expected.left);
        assert_eq!(rect.bottom - rect.top, expected.bottom - expected.top);
    }
}

#[test]
fn ordinary_root_and_tray_submenu_keep_top_edge_alignment() {
    let owner = Owner::new();
    let work = work_area();
    let point = POINT {
        x: work.left + 8,
        y: work.top + (work.bottom - work.top) / 2,
    };
    let ordinary = Cascade::open(owner.0, tree(), point).unwrap();
    let rect = bounds(&ordinary, 0);
    assert_eq!((rect.left, rect.top), (point.x, point.y));
    drop(ordinary);
    let mut parent = tree().remove(0);
    parent.children = Some(tree());
    let mut menu = Cascade::open_with_tray_mode(owner.0, vec![parent], point, true).unwrap();
    let root = bounds(&menu, 0);
    assert_eq!(root.bottom, point.y);
    menu.event(0, Event::Move(8, 8)).unwrap();
    assert_eq!(menu.pages.len(), 2);
    let child = bounds(&menu, 1);
    assert_eq!(child.top, root.top);
    assert_eq!(child.left, root.right);
    assert_inside(child, work);
    let handles: Vec<_> = menu
        .pages
        .iter()
        .map(|p| p.panel.handle().unwrap())
        .collect();
    menu.cancel();
    for hwnd in handles {
        assert!(!unsafe { IsWindow(Some(hwnd)).as_bool() });
    }
}

#[test]
fn compact_policy_and_checklist_keep_geometry_selection_and_window() {
    let owner = Owner::new();
    let point = POINT { x: 30, y: 60 };
    let rows = || {
        let mut rows = tree();
        rows.truncate(1);
        rows[0].label = "接收".into();
        rows
    };
    let compact = Cascade::open_sized(owner.0, rows(), point, false, 120).unwrap();
    let rect = bounds(&compact, 0);
    let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(owner.0) }.clamp(48, 768);
    assert!(rect.right - rect.left <= (140 * dpi / 96) as i32);
    drop(compact);
    let mut menu = Cascade::open_checklist(owner.0, rows(), point).unwrap();
    let hwnd = menu.pages[0].panel.handle().unwrap();
    let rect = bounds(&menu, 0);
    assert_eq!((rect.left, rect.top), (point.x, point.y));
    menu.key(0x28).unwrap();
    menu.key(0x0d).unwrap();
    assert_eq!(menu.take_command(), Some(rows()[0].command));
    assert!(!menu.closed());
    let mut checked = rows();
    checked[0].checked = true;
    menu.replace_checklist(checked).unwrap();
    assert_eq!(menu.pages[0].panel.handle().unwrap(), hwnd);
    assert!(menu.tree[0].checked);
    let mut refreshed = rows();
    refreshed[0].checked = true;
    refreshed.push(rows().remove(0));
    refreshed[1].command += 1;
    refreshed[1].label = "第二台".into();
    menu.replace_checklist(refreshed).unwrap();
    assert_eq!(menu.pages[0].panel.handle().unwrap(), hwnd);
    assert!(menu.tree[0].checked);
    assert_eq!(bounds(&menu, 0).top, point.y);
}
