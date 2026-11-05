//! 隐藏钉图面板的真实 BUTTON 生命周期；不注册全局输入或显示前台窗口。
use super::*;

struct Fixture {
    panel: Window,
    _owner: Window,
    state: Box<State>,
}
impl Fixture {
    unsafe fn new(pinned: bool) -> Self {
        let state = Box::new(State::new(None, false).unwrap());
        state.canvas.borrow_mut().pinned = pinned;
        let owner = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("hidden content fixture"),
                WS_POPUP,
                0,
                0,
                640,
                480,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        SetWindowLongPtrW(owner.0, GWLP_USERDATA, (&*state as *const State) as isize);
        let panel = create(owner.0, &state, false).unwrap();
        Self {
            panel,
            _owner: owner,
            state,
        }
    }
}

#[test]
fn hidden_pin_preserves_panel_owner_without_allocating_unused_buttons() {
    unsafe {
        let fixture = Fixture::new(true);
        assert!(IsWindow(Some(fixture.panel.0)).as_bool());
        assert_eq!(fixture.state.tool_panel.get(), fixture.panel.0);
        assert_eq!(
            GetWindowLongPtrW(fixture.panel.0, GWLP_USERDATA),
            fixture._owner.0 .0 as isize
        );
        assert!(!IsWindowVisible(fixture.panel.0).as_bool());
        assert!(
            entries().all(|(id, _)| GetDlgItem(Some(fixture.panel.0), id as i32).is_err()),
            "hidden pin allocated unused native BUTTON controls"
        );
        let panel = fixture.panel.0;
        drop(fixture);
        assert!(!IsWindow(Some(panel)).as_bool());
    }
}

unsafe fn handles(panel: HWND) -> Vec<HWND> {
    entries()
        .map(|(id, _)| GetDlgItem(Some(panel), id as i32).unwrap())
        .collect()
}

#[test]
fn ordinary_hidden_workspace_keeps_prepared_controls_before_transaction_commit() {
    unsafe {
        let fixture = Fixture::new(false);
        assert!(content::ready(fixture.panel.0));
        assert_eq!(handles(fixture.panel.0).len(), entries().count());
        assert!(!IsWindowVisible(fixture.panel.0).as_bool());
    }
}

#[test]
fn first_pin_materialization_projects_current_style_and_busy_gate_once_and_releases_children() {
    unsafe {
        let fixture = Fixture::new(true);
        let style = fixture
            .state
            .canvas
            .borrow()
            .editor
            .style()
            .with_width(42.0)
            .unwrap();
        fixture.state.canvas.borrow_mut().editor.set_style(style);
        fixture.state.file_dialog.set(true);
        invalidate(&fixture.state);
        refresh(fixture._owner.0);
        assert_eq!(fixture.state.tool_panel_style.get(), None);
        prepare(fixture.panel.0, &fixture.state).unwrap();
        assert_eq!(fixture.state.tool_panel_style.get(), Some(style));
        verify_captions(fixture.panel.0, &fixture.state).unwrap();
        let button = GetDlgItem(Some(fixture.panel.0), 0x53).unwrap();
        assert!(!IsWindowEnabled(button).as_bool());
        let controls = handles(fixture.panel.0);
        prepare(fixture.panel.0, &fixture.state).unwrap();
        assert_eq!(handles(fixture.panel.0), controls);
        fixture.state.file_dialog.set(false);
        refresh(fixture._owner.0);
        assert!(IsWindowEnabled(button).as_bool());
        let panel = fixture.panel.0;
        drop(fixture);
        assert!(!IsWindow(Some(panel)).as_bool());
        assert!(controls
            .into_iter()
            .all(|hwnd| !IsWindow(Some(hwnd)).as_bool()));
    }
}

#[test]
fn pin_materialization_uses_current_hidden_scroll_geometry() {
    unsafe {
        let fixture = Fixture::new(true);
        let panel = fixture.panel.0;
        SetWindowPos(
            panel,
            None,
            0,
            0,
            220,
            150,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        windows::Win32::UI::Controls::SetScrollPos(panel, SB_HORZ, 150, true);
        windows::Win32::UI::Controls::SetScrollPos(panel, SB_VERT, 100, true);
        prepare(panel, &fixture.state).unwrap();
        let offset = scroll::sync(panel);
        assert!(offset.x > 0 && offset.y > 0);
        for (index, (id, _)) in entries().enumerate() {
            let expected = geometry::button(
                index,
                super::super::tool_commands::TOOLS.len(),
                GetDpiForWindow(panel),
            );
            let mut rect = RECT::default();
            GetWindowRect(GetDlgItem(Some(panel), id as i32).unwrap(), &mut rect).unwrap();
            let mut origin = POINT {
                x: rect.left,
                y: rect.top,
            };
            assert!(ScreenToClient(panel, &mut origin).as_bool());
            assert_eq!(
                (origin.x, origin.y),
                (expected.left - offset.x, expected.top - offset.y)
            );
        }
    }
}

#[test]
fn failed_partial_pin_materialization_rolls_back_and_retries_without_duplicate_ids() {
    unsafe {
        let fixture = Fixture::new(true);
        let mut created = Vec::new();
        let result = content::prepare_with(fixture.panel.0, &fixture.state, || {
            children::create_checked(
                fixture.panel.0,
                HINSTANCE(GetModuleHandleW(None).unwrap().0),
                |index| {
                    if index == 3 {
                        created = entries()
                            .take(3)
                            .map(|(id, _)| GetDlgItem(Some(fixture.panel.0), id as i32).unwrap())
                            .collect();
                        return Err(windows::core::Error::new(
                            E_OUTOFMEMORY,
                            "injected child creation failure",
                        ));
                    }
                    Ok(())
                },
            )
        });
        assert!(result.is_err());
        assert_eq!(created.len(), 3);
        assert!(created
            .into_iter()
            .all(|hwnd| !IsWindow(Some(hwnd)).as_bool()));
        assert!(!content::ready(fixture.panel.0));
        assert!(entries().all(|(id, _)| GetDlgItem(Some(fixture.panel.0), id as i32).is_err()));
        prepare(fixture.panel.0, &fixture.state).unwrap();
        assert_eq!(handles(fixture.panel.0).len(), entries().count());
    }
}

#[test]
fn showwindow_position_message_prepares_controls_and_reentrant_show_stays_hidden() {
    unsafe {
        let fixture = Fixture::new(true);
        content::prepare_with(fixture.panel.0, &fixture.state, || {
            let mut position = WINDOWPOS {
                hwnd: fixture.panel.0,
                flags: SWP_SHOWWINDOW,
                ..Default::default()
            };
            content::before_show(fixture.panel.0, &fixture.state, &mut position);
            assert!(!position.flags.contains(SWP_SHOWWINDOW));
            assert!(position.flags.contains(SWP_HIDEWINDOW));
            children::create(
                fixture.panel.0,
                HINSTANCE(GetModuleHandleW(None).unwrap().0),
            )
        })
        .unwrap();
        assert!(content::ready(fixture.panel.0));
        let fixture = Fixture::new(true);
        let mut position = WINDOWPOS {
            hwnd: fixture.panel.0,
            flags: SWP_SHOWWINDOW,
            ..Default::default()
        };
        SendMessageW(
            fixture.panel.0,
            WM_WINDOWPOSCHANGING,
            None,
            Some(LPARAM((&mut position as *mut WINDOWPOS) as isize)),
        );
        assert!(content::ready(fixture.panel.0));
        assert!(position.flags.contains(SWP_SHOWWINDOW));
        assert_eq!(handles(fixture.panel.0).len(), entries().count());
        assert!(!IsWindowVisible(fixture.panel.0).as_bool());
    }
}

#[test]
fn actual_showwindow_cancels_reentrant_display_then_retries_with_real_button_state() {
    unsafe {
        let fixture = Fixture::new(true);
        let panel = fixture.panel.0;
        // 自有面板在屏幕外、非前台展示；不激活 owner、不发送全局输入。
        SetWindowPos(
            panel,
            None,
            -30000,
            -30000,
            220,
            150,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        let failure = content::prepare_with(panel, &fixture.state, || {
            let _ = ShowWindow(panel, SW_SHOWNOACTIVATE);
            assert!(
                !IsWindowVisible(panel).as_bool(),
                "reentrant show exposed an incomplete panel"
            );
            Err(windows::core::Error::new(
                E_OUTOFMEMORY,
                "injected preparation failure after reentrant show",
            ))
        });
        assert!(failure.is_err());
        assert!(!content::ready(panel));
        fixture.state.file_dialog.set(true);
        let style = fixture
            .state
            .canvas
            .borrow()
            .editor
            .style()
            .with_width(42.0)
            .unwrap();
        fixture.state.canvas.borrow_mut().editor.set_style(style);
        let _ = ShowWindow(panel, SW_SHOWNOACTIVATE);
        assert!(IsWindowVisible(panel).as_bool());
        assert!(content::ready(panel));
        verify_captions(panel, &fixture.state).unwrap();
        assert!(!IsWindowEnabled(GetDlgItem(Some(panel), 0x53).unwrap()).as_bool());
        let controls = handles(panel);
        let _ = ShowWindow(panel, SW_HIDE);
        let _ = ShowWindow(panel, SW_SHOWNOACTIVATE);
        assert!(IsWindowVisible(panel).as_bool());
        assert_eq!(handles(panel), controls);
        let _ = ShowWindow(panel, SW_HIDE);
        drop(fixture);
        assert!(!IsWindow(Some(panel)).as_bool());
        assert!(controls
            .into_iter()
            .all(|hwnd| !IsWindow(Some(hwnd)).as_bool()));
    }
}
