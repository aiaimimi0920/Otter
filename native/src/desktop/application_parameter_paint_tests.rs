//! 在未切换desktop核正常Application调度是否允许每个参数child完成初始paint。
use super::*;
use windows::Win32::Graphics::Gdi::GetUpdateRect;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, SetFocus};

#[test]
fn application_pump_paints_parameter_children_without_rebuilding_them() {
    unsafe { check_pump(false) }
}
#[test]
fn host_focus_then_panel_entry_does_not_starve_other_parameter_children() {
    unsafe { check_pump(true) }
}
unsafe fn check_pump(focused: bool) {
    unsafe {
        let mut app = app();
        let host = app.documents[0].window.0;
        app.documents[0].observe_priority().unwrap();
        let _ = ShowWindow(host, SW_SHOWNOACTIVATE);
        let _ = ShowWindow(app.documents[0].panel.0, SW_SHOWNOACTIVATE);
        if focused {
            let _ = SetFocus(Some(host));
            assert_eq!(GetFocus(), host);
        }
        app.pending_windows(true).unwrap();
        app.documents[0].state.parameters_ui.toggle.set(true);
        app.pending_windows(true).unwrap();
        let panel = app.documents[0].parameters.as_ref().unwrap().handle();
        if focused {
            assert_eq!(GetFocus(), GetDlgItem(Some(panel), 1001).unwrap());
        }
        let mut child = GetWindow(panel, GW_CHILD).unwrap();
        let mut children = Vec::new();
        while !child.is_invalid() {
            children.push(child);
            child = GetWindow(child, GW_HWNDNEXT).unwrap_or_default();
        }
        assert!(children.len() >= 6);
        let mut painted = std::collections::HashSet::new();
        let mut counts = std::collections::BTreeMap::new();
        let mut paints = std::collections::BTreeMap::new();
        let mut message = MSG::default();
        for _ in 0..512 {
            if !PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE).as_bool() {
                break;
            }
            assert_eq!(app.next_live_message(&mut message).unwrap(), 1);
            *counts.entry(message.message).or_insert(0) += 1;
            if message.message == WM_PAINT {
                painted.insert(message.hwnd.0 as usize);
                *paints.entry(message.hwnd.0 as usize).or_insert(0) += 1;
            }
            if !app.translate_workspace_chrome(&message)
                && !app
                    .documents
                    .iter()
                    .any(|d| d.parameters.as_ref().is_some_and(|p| p.translate(&message)))
                && !app
                    .documents
                    .iter()
                    .any(|d| crate::desktop::tool_panel::translate(&d.state, &message))
            {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            app.pending_windows(true).unwrap();
            app.pending_groups(true).unwrap();
        }
        eprintln!("paint message counts: {counts:?}");
        for (raw, count) in paints {
            let hwnd = HWND(raw as *mut _);
            let mut class = [0u16; 128];
            let n = GetClassNameW(hwnd, &mut class).max(0) as usize;
            eprintln!(
                "paint hwnd={raw} count={count} class={} host={} parameter={}",
                String::from_utf16_lossy(&class[..n]),
                hwnd == host,
                hwnd == panel
            );
        }
        eprintln!(
            "host render_active={} render_deferred={} invalid={}",
            app.documents[0].state.render_active.get(),
            app.documents[0].state.render_deferred.get(),
            GetUpdateRect(host, None, false).as_bool()
        );
        let unpainted: Vec<_> = children
            .iter()
            .filter(|h| !painted.contains(&(h.0 as usize)))
            .map(|h| GetDlgCtrlID(*h))
            .collect();
        let invalid: Vec<_> = children
            .iter()
            .filter(|h| GetUpdateRect(**h, None, false).as_bool())
            .map(|h| GetDlgCtrlID(*h))
            .collect();
        assert!(
            children.iter().all(|h| IsWindow(Some(*h)).as_bool()),
            "children rebuilt"
        );
        app.shutdown();
        assert!(
            unpainted.is_empty(),
            "children without paint: {unpainted:?}; still invalid: {invalid:?}"
        );
        assert!(invalid.is_empty(), "children still invalid: {invalid:?}");
    }
}
