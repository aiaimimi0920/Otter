//! 同一控件实现的child宿主：真实编辑/类型/IME/清理，不代普通桌面或人工输入法验收。
use super::*;

unsafe fn binding(panel: &Panel, key: &str) -> controls::Binding {
    panel
        .ui
        .bindings
        .borrow()
        .iter()
        .find(|b| b.key == key)
        .unwrap()
        .clone()
}
#[test]
fn embedded_field_is_a_real_child_and_does_not_replace_the_full_panel_owner() {
    unsafe {
        let f = Fixture::new();
        let panel = Panel::embedded(f.host.0, &f.state, f.panel.handle(), "text").unwrap();
        assert_eq!(GetParent(panel.handle()).unwrap(), f.panel.handle());
        assert_eq!(f.state.parameters_ui.window(), f.panel.handle());
        assert_eq!(panel.ui.model.borrow().rows.len(), 1);
        assert_eq!(panel.ui.model.borrow().rows[0].id, "text");
        assert!(panel
            .ui
            .bindings
            .borrow()
            .iter()
            .all(|b| b.key.starts_with("text:")));
        assert!(panel.ui.bindings.borrow().len() < 8);
        assert!(panel.embedded_height() < 240);
        assert!(Panel::embedded(f.host.0, &f.state, f.panel.handle(), "secret").is_err());
        assert!(Panel::embedded(f.host.0, &f.state, f.panel.handle(), "missing").is_err());
        let hwnd = panel.handle();
        drop(panel);
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert_eq!(f.state.parameters_ui.window(), f.panel.handle());
    }
}
#[test]
fn all_public_widget_families_reuse_the_real_single_row_controls() {
    unsafe {
        let f = Fixture::with_parameters(serde_json::json!([
            {"id":"text","widget":"text","default":"文字"},
            {"id":"path","widget":"path","default":"example.txt"},
            {"id":"body","widget":"textarea","default":"第一行\n第二行"},
            {"id":"number","widget":"number","default":2},
            {"id":"slider","widget":"slider","default":3,"min":0,"max":10},
            {"id":"switch","widget":"switch","default":false},
            {"id":"radio","widget":"radio","default":2,"options":[{"label":"二","value":2}]},
            {"id":"color","widget":"color","default":"#123456"},
            {"id":"file","widget":"file","default":""},
            {"id":"image","widget":"image_link","default":""}
        ]));
        for (id, key) in [
            ("text", "edit"),
            ("path", "edit"),
            ("body", "edit"),
            ("number", "edit"),
            ("slider", "slider"),
            ("switch", "boolean"),
            ("radio", "options"),
            ("color", "color"),
            ("file", "file"),
            ("image", "file"),
        ] {
            let panel = Panel::embedded(f.host.0, &f.state, f.panel.handle(), id).unwrap();
            assert!(IsWindow(Some(binding(&panel, &format!("{id}:{key}")).hwnd)).as_bool());
            assert!(panel.exit_idle(), "{id}");
            assert!(panel.ui.bindings.borrow().len() < 8);
            assert!(panel.embedded_height() <= 240, "{id}");
        }
    }
}
#[test]
fn embedded_text_option_and_boolean_use_the_original_candidate_transactions() {
    unsafe {
        let f = Fixture::new();
        for (field, key) in [
            ("text", "text:edit"),
            ("mode", "mode:options"),
            ("enabled", "enabled:boolean"),
        ] {
            let panel = Panel::embedded(f.host.0, &f.state, f.panel.handle(), field).unwrap();
            let b = binding(&panel, key);
            match field {
                "text" => {
                    SetWindowTextW(b.hwnd, &HSTRING::from("内嵌🙂")).unwrap();
                    controls::commit(&panel.ui, &b);
                }
                "mode" => {
                    SendMessageW(b.hwnd, CB_SETCURSEL, Some(WPARAM(1)), None);
                    SendMessageW(
                        panel.handle(),
                        WM_COMMAND,
                        Some(WPARAM((CBN_SELCHANGE as usize) << 16)),
                        Some(LPARAM(b.hwnd.0 as isize)),
                    );
                }
                _ => {
                    SendMessageW(b.hwnd, BM_CLICK, None, None);
                }
            }
            assert!(!panel.exit_idle());
            panel.update(&f.state).unwrap();
            assert!(panel.take_commit());
            assert!(!panel.take_commit());
        }
        assert_eq!(f.value("text"), "内嵌🙂");
        assert_eq!(f.value("mode"), 2);
        assert_eq!(f.value("enabled"), true);
    }
}
#[test]
fn linked_embedded_value_is_effective_read_only_and_does_not_replace_the_manual_value() {
    unsafe {
        let f = Fixture::new();
        desktop::parameter_view::View::update(
            &f.state,
            serde_json::from_value(serde_json::json!({"n":8})).unwrap(),
            vec!["n".into()],
        );
        let panel = Panel::embedded(f.host.0, &f.state, f.panel.handle(), "n").unwrap();
        let b = binding(&panel, "n:edit");
        assert!(!IsWindowEnabled(b.hwnd).as_bool());
        assert_eq!(controls::text(b.hwnd).unwrap(), "8");
        SetWindowTextW(b.hwnd, &HSTRING::from("6")).unwrap();
        controls::commit(&panel.ui, &b);
        panel.update(&f.state).unwrap();
        assert_eq!(f.value("n"), 3);
        assert!(!panel.take_commit());
    }
}
#[test]
fn child_ime_and_dpi_keep_draft_then_parent_destruction_reclaims_every_handle() {
    unsafe {
        let f = Fixture::new();
        let panel = Panel::embedded(f.host.0, &f.state, f.panel.handle(), "text").unwrap();
        let b = binding(&panel, "text:edit");
        SetWindowTextW(b.hwnd, &HSTRING::from("未完成🙂")).unwrap();
        SendMessageW(b.hwnd, WM_IME_STARTCOMPOSITION, None, None);
        controls::commit(&panel.ui, &b);
        assert!(panel.ui.pending.borrow().is_empty());
        SendMessageW(panel.handle(), WM_DPICHANGED_AFTERPARENT, None, None);
        assert_eq!(controls::text(b.hwnd).unwrap(), "未完成🙂");
        assert!(keyboard::composing(b.hwnd));
        assert!(!panel.exit_idle());
        let handles: Vec<_> = panel
            .ui
            .placements
            .borrow()
            .iter()
            .map(|(h, _)| *h)
            .collect();
        DestroyWindow(f.panel.handle()).unwrap();
        assert!(panel.retired());
        assert!(handles.into_iter().all(|h| !IsWindow(Some(h)).as_bool()));
        assert_eq!(f.value("text"), "原值");
    }
}
#[test]
fn retired_child_cannot_publish_even_when_host_and_configuration_stay_current() {
    unsafe {
        let f = Fixture::new();
        let panel = Panel::embedded(f.host.0, &f.state, f.panel.handle(), "mode").unwrap();
        let b = binding(&panel, "mode:options");
        let controls::Kind::Options(snapshot, choices) = b.kind else {
            panic!("expected options");
        };
        let calls = Cell::new(0);
        let before = Stamp::capture(&f.state);
        assert!(!snapshot
            .submit_guarded(f.host.0, &f.state, choices[1], None, &|| {
                calls.set(calls.get() + 1);
                if calls.get() == 2 {
                    panel.retire_embedded();
                }
                panel.ui.available()
            })
            .unwrap());
        assert_eq!(calls.get(), 2);
        assert!(panel.retired());
        assert!(before == Stamp::capture(&f.state));
        assert_eq!(f.value("mode"), "fast");
    }
}
