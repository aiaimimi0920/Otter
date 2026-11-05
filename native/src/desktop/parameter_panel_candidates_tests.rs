//! 真实隐藏HWND候选选择与有界绘制；不抢前台、不访问外部数据、不伪造正式输出。
use super::*;
use serde_json::json;
fn fixture() -> Fixture {
    unsafe {
        Fixture::with_parameters(json!([
            {"id":"result_index","type":"number","default":0,"min":0,"max":10,"step":1},
            {"id":"query","type":"string","default":"原查询"}
        ]))
    }
}
fn deliver(f: &Fixture) {
    let request = crate::graph_requests::Request {
        id: crate::workspace_id::UnitId::parse("candidate-result").unwrap(),
        generation: 1,
    };
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let input = desktop::loom_inline::prepare(
        1,
        1,
        vec![0, 0, 255, 255].into(),
        &crate::editor::Editor::default(),
        &cancel,
    )
    .unwrap();
    let crate::loom_art_wire::Input::InlinePng { base64, .. } = input.descriptor() else {
        panic!("png")
    };
    let metadata = json!({"items":[{"index":0,"title":"红色","thumbnail":format!("data:image/png;base64,{base64}")},
        {"index":2,"title":"第二个候选"}],"selectedIndex":0});
    let batch = desktop::loom_output_batch::decode(
        &json!({"output":{"kind":"value","value":42}}),
        Some(&metadata),
        &cancel,
    )
    .unwrap();
    let mut results = f.state.execution_results.borrow_mut();
    results.begin(request.clone());
    results.finish(&request, batch).unwrap();
    results.receive_candidates(Some(metadata));
    results.bind_candidate_configuration(Stamp::capture(&f.state));
    drop(results);
    unsafe {
        f.tick();
    }
}
#[test]
fn actual_candidate_button_sets_typed_index_and_requests_one_manual_execution_without_publishing_thumbnail(
) {
    unsafe {
        let f = fixture();
        deliver(&f);
        let output = f.state.execution_results.borrow().output_stamp();
        f.click("candidate:2");
        assert!(!f.panel.exit_idle());
        assert_eq!(f.panel.ui.pending.borrow().len(), 1);
        f.tick();
        assert_eq!(f.value("result_index"), 2);
        assert_eq!(
            f.state.pending_start.borrow().as_ref().unwrap().trigger,
            crate::graph_execution_config::Trigger::Manual
        );
        assert!(output.matches(&f.state.execution_results.borrow()));
        assert!(f.state.execution_results.borrow().display().is_none());
        assert!(f
            .panel
            .ui
            .bindings
            .borrow()
            .iter()
            .all(|b| !b.key.starts_with("candidate:")));
    }
}
#[test]
fn stale_same_index_set_configuration_and_locked_owner_cannot_select() {
    unsafe {
        let f = fixture();
        deliver(&f);
        let controls::Kind::Candidate(old) = f.binding("candidate:2").kind else {
            panic!("candidate")
        };
        deliver(&f);
        assert!(!old.submit(f.host.0, &f.state, &|| true).unwrap());
        let controls::Kind::Candidate(current) = f.binding("candidate:2").kind else {
            panic!("candidate")
        };
        f.state.group_locked.set(true);
        assert!(!current.submit(f.host.0, &f.state, &|| true).unwrap());
        f.state.group_locked.set(false);
        f.state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .parameters
            .set("query", Some(json!("新查询")))
            .unwrap();
        f.tick();
        assert!(!current.submit(f.host.0, &f.state, &|| true).unwrap());
        assert!(f
            .panel
            .ui
            .bindings
            .borrow()
            .iter()
            .all(|b| !b.key.starts_with("candidate:")));
        assert_eq!(f.value("result_index"), 0);
        assert!(f.state.pending_start.borrow().is_none());
    }
}
#[test]
fn dirty_draft_and_linked_index_are_not_silently_reauthorized_for_candidate_selection() {
    unsafe {
        let f = fixture();
        deliver(&f);
        f.text("query:edit", "未提交查询");
        f.click("candidate:2");
        assert!(f.panel.ui.pending.borrow().is_empty());
        assert_eq!(f.value("query"), "原查询");
        assert!(!f.panel.exit_idle());
        f.click("reload");
        f.tick();
        let mut values = serde_json::Map::new();
        values.insert("result_index".into(), json!(0));
        desktop::parameter_view::View::update(&f.state, values, vec!["result_index".into()]);
        f.tick();
        let binding = f.binding("candidate:2");
        assert!(!binding.enabled && !IsWindowEnabled(binding.hwnd).as_bool());
        f.click("candidate:2");
        f.tick();
        assert_eq!(f.value("result_index"), 0);
        assert!(f.state.pending_start.borrow().is_none());
    }
}
#[test]
fn candidate_queued_before_new_draft_or_ime_does_not_commit_or_run() {
    unsafe {
        for composing in [false, true] {
            let f = fixture();
            deliver(&f);
            f.click("candidate:2");
            assert_eq!(f.panel.ui.pending.borrow().len(), 1);
            let edit = f.binding("query:edit").hwnd;
            if composing {
                SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
            } else {
                f.text("query:edit", "入队后新草稿");
            }
            f.tick();
            assert_eq!(f.value("result_index"), 0);
            assert_eq!(f.value("query"), "原查询");
            assert!(f.state.pending_start.borrow().is_none());
            assert!(f.panel.ui.pending.borrow().is_empty());
            assert!(!f.panel.exit_idle());
            assert!(f.panel.ui.message.borrow().contains("候选或参数已变化"));
            if !composing {
                assert_eq!(controls::text(edit).unwrap(), "入队后新草稿");
            }
        }
    }
}
#[test]
fn candidate_outside_numeric_spec_or_with_text_index_is_disabled_without_clamping() {
    unsafe {
        for parameter in [
            json!({"id":"result_index","type":"number","default":0,"min":0,"max":1}),
            json!({"id":"result_index","type":"string","default":"0"}),
        ] {
            let expected = parameter["default"].clone();
            let f = Fixture::with_parameters(json!([parameter]));
            deliver(&f);
            let binding = f.binding("candidate:2");
            assert!(!binding.enabled && !IsWindowEnabled(binding.hwnd).as_bool());
            let controls::Kind::Candidate(choice) = binding.kind else {
                panic!("candidate")
            };
            assert!(!choice.submit(f.host.0, &f.state, &|| true).unwrap());
            f.click("candidate:2");
            f.tick();
            assert_eq!(f.value("result_index"), expected);
            assert!(f.state.pending_start.borrow().is_none());
        }
    }
}
#[test]
fn candidate_standard_button_space_uses_panel_keyboard_pipeline_once() {
    unsafe {
        let f = fixture();
        deliver(&f);
        let _ = ShowWindow(f.host.0, SW_SHOWNOACTIVATE);
        f.panel.show();
        let button = f.binding("candidate:2").hwnd;
        let _ = SetFocus(Some(button));
        assert_eq!(GetFocus(), button);
        for message in [WM_KEYDOWN, WM_KEYUP] {
            let msg = MSG {
                hwnd: button,
                message,
                wParam: WPARAM(VK_SPACE.0 as usize),
                ..Default::default()
            };
            if !f.panel.translate(&msg) {
                DispatchMessageW(&msg);
            }
        }
        assert_eq!(f.panel.ui.pending.borrow().len(), 1);
        f.tick();
        assert_eq!(f.value("result_index"), 2);
        assert_eq!(
            f.state.pending_start.borrow().as_ref().unwrap().trigger,
            crate::graph_execution_config::Trigger::Manual
        );
    }
}
#[test]
fn new_execution_and_result_without_candidates_retire_old_picker_identity() {
    unsafe {
        let f = fixture();
        deliver(&f);
        let stamp = Stamp::capture(&f.state);
        let request = crate::graph_requests::Request {
            id: crate::workspace_id::UnitId::parse("new-result").unwrap(),
            generation: 2,
        };
        f.state
            .execution_results
            .borrow_mut()
            .begin(request.clone());
        assert!(f
            .state
            .execution_results
            .borrow()
            .candidate_view(&stamp)
            .is_none());
        assert!(f.state.execution_results.borrow().candidates().is_none());
        let batch = desktop::loom_output_batch::decode(
            &json!({"output":{"kind":"value","value":7}}),
            None,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        f.state
            .execution_results
            .borrow_mut()
            .finish(&request, batch)
            .unwrap();
        f.tick();
        assert!(f
            .panel
            .ui
            .bindings
            .borrow()
            .iter()
            .all(|b| !b.key.starts_with("candidate:")));
    }
}
#[test]
fn selected_same_index_can_explicitly_run_once_and_retired_panel_cannot_queue() {
    unsafe {
        let f = fixture();
        deliver(&f);
        f.click("candidate:0");
        f.click("candidate:2");
        f.tick();
        assert_eq!(f.value("result_index"), 0);
        assert!(f.state.pending_start.borrow().is_some());
        f.panel.ui.close();
        f.click("candidate:2");
        assert!(f.panel.ui.pending.borrow().is_empty());
    }
}
#[test]
fn candidate_owner_draw_uses_prepared_pixels_and_releases_its_gdi_objects() {
    unsafe {
        let f = fixture();
        deliver(&f);
        let binding = f.binding("candidate:0");
        let dc = CreateCompatibleDC(None);
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 202,
                biHeight: -80,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
        let old = SelectObject(dc, HGDIOBJ(bitmap.0));
        let item = DRAWITEMSTRUCT {
            hwndItem: binding.hwnd,
            hDC: dc,
            rcItem: RECT {
                right: 202,
                bottom: 80,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(controls::candidates::draw(&f.panel.ui, &item));
        assert!(GdiFlush().as_bool());
        let pixels = std::slice::from_raw_parts(bits as *const u8, 202 * 80 * 4);
        assert!(pixels.chunks_exact(4).any(|p| p[..3] == [0, 0, 255]));
        SelectObject(dc, old);
        assert!(DeleteObject(HGDIOBJ(bitmap.0)).as_bool());
        assert!(DeleteDC(dc).as_bool());
    }
}
