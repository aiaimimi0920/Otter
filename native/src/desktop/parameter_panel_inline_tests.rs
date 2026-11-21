//! 多行真实 EDIT/COMBOBOX 与原事务；几何测试只操作自有 HWND，不触碰用户桌面。
use super::*;
unsafe fn fixture(parameters: Option<serde_json::Value>) -> (Fixture, desktop::Window) {
    let mut f = parameters.map_or_else(|| Fixture::new(), |p| Fixture::with_parameters(p));
    let parent = desktop::Window(
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            None,
            WS_POPUP,
            0,
            0,
            256,
            300,
            None,
            None,
            None,
            None,
        )
        .unwrap(),
    );
    f.panel = Panel::inline(f.host.0, &f.state, parent.0).unwrap();
    place(&f, 0);
    (f, parent)
}
unsafe fn place(f: &Fixture, offset: i32) {
    let rows = f
        .panel
        .ui
        .model
        .borrow()
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let rect = RECT {
                left: 0,
                top: i as i32 * 36 - offset,
                right: 228,
                bottom: i as i32 * 36 + 28 - offset,
            };
            (
                row.id.clone(),
                rect,
                RECT {
                    top: rect.top.max(0),
                    bottom: rect.bottom.min(100),
                    ..rect
                },
            )
        })
        .collect();
    f.panel
        .inline_binding()
        .unwrap()
        .place(
            RECT {
                right: 256,
                bottom: 300,
                ..Default::default()
            },
            rows,
        )
        .unwrap();
}
#[test]
fn inline_controls_share_one_owner_clip_actual_values_and_preserve_drafts_while_scrolling() {
    unsafe {
        let (f, parent) = fixture(None);
        assert_eq!(GetParent(f.panel.handle()).unwrap(), parent.0);
        assert_eq!(f.state.parameters_ui.capture_window(), parent.0);
        assert_eq!(
            GetWindowLongW(f.panel.handle(), GWL_STYLE) as u32 & WS_CAPTION.0,
            0
        );
        let edit = f.text("text:edit", "未提交🙂");
        let mut before = RECT::default();
        GetWindowRect(edit, &mut before).unwrap();
        let region = CreateRectRgn(0, 0, 0, 0);
        assert_ne!(GetWindowRgn(f.panel.handle(), region), RGN_ERROR);
        let mut local = POINT {
            x: before.left + 1,
            y: before.bottom + 1,
        };
        ScreenToClient(f.panel.handle(), &mut local).ok().unwrap();
        assert!(
            !PtInRegion(region, local.x, local.y).as_bool(),
            "control backdrop must stop at its actual bottom"
        );
        DeleteObject(HGDIOBJ(region.0)).unwrap();
        place(&f, 13);
        assert_eq!(f.binding("text:edit").hwnd, edit);
        assert_eq!(controls::text(edit).unwrap(), "未提交🙂");
        let mut after = RECT::default();
        GetWindowRect(edit, &mut after).unwrap();
        assert_eq!(after.top, before.top - 13);
        let region = CreateRectRgn(0, 0, 0, 0);
        assert_ne!(GetWindowRgn(f.panel.handle(), region), RGN_ERROR);
        assert!(
            !PtInRegion(region, 82, 2).as_bool(),
            "eye slot must remain in the capsule parent"
        );
        assert!(
            !PtInRegion(region, 105, 20).as_bool(),
            "row gap must pass through"
        );
        DeleteObject(HGDIOBJ(region.0)).unwrap();
        controls::commit(&f.panel.ui, &f.binding("text:edit"));
        f.tick();
        assert_eq!(f.value("text"), "未提交🙂");
    }
}
#[test]
fn all_inline_drafts_commit_atomically_beyond_the_old_sixteen_row_page() {
    unsafe {
        let params = (0..32)
            .map(|i| serde_json::json!({"id":format!("p{i}"),"type":"string","default":"old"}))
            .collect::<Vec<_>>();
        let (f, _parent) = fixture(Some(params.into()));
        assert_eq!(f.panel.ui.bindings.borrow().len(), 32);
        for i in 0..32 {
            f.text(&format!("p{i}:edit"), &format!("new-{i}"));
        }
        controls::commit(&f.panel.ui, &f.binding("p0:edit"));
        f.tick();
        for i in 0..32 {
            assert_eq!(f.value(&format!("p{i}")), format!("new-{i}"));
        }
        assert!(f.panel.exit_idle());
    }
}
#[test]
fn inline_typed_choices_disable_restore_and_color_clear_reuse_original_actions() {
    unsafe {
        let params = serde_json::json!([
            {"id":"mode","widget":"select","options":["2",2],"default":"2"},
            {"id":"text","type":"string","default":"fallback"},
            {"id":"color","widget":"color","default":"#112233"}
        ]);
        let (f, _parent) = fixture(Some(params));
        let combo = f.binding("mode:options").hwnd;
        SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(1)), None);
        SendMessageW(
            f.panel.handle(),
            WM_COMMAND,
            Some(WPARAM((CBN_SELCHANGE as usize) << 16)),
            Some(LPARAM(combo.0 as isize)),
        );
        f.tick();
        assert_eq!(f.value("mode"), 2);
        f.text("text:edit", "stashed");
        controls::commit(&f.panel.ui, &f.binding("text:edit"));
        f.tick();
        f.panel.inline_binding().unwrap().context("text", false);
        f.tick();
        assert_eq!(f.value("text"), crate::graph_params::DISABLED);
        f.panel.inline_binding().unwrap().context("text", false);
        f.tick();
        assert_eq!(f.value("text"), "stashed");
        f.panel.inline_binding().unwrap().context("color", true);
        f.tick();
        assert_eq!(f.value("color"), "");
    }
}

#[test]
fn queued_text_then_slider_tracking_keeps_the_slider_draft_committable() {
    unsafe {
        let (f, _parent) = fixture(None);
        f.text("text:edit", "已排队");
        controls::commit(&f.panel.ui, &f.binding("text:edit"));
        let slider = f.binding("n:slider").hwnd;
        SendMessageW(slider, TBM_SETPOS, Some(WPARAM(0)), Some(LPARAM(7000)));
        SendMessageW(
            f.panel.handle(),
            WM_HSCROLL,
            Some(WPARAM(TB_THUMBTRACK as usize)),
            Some(LPARAM(slider.0 as isize)),
        );
        f.tick();
        assert_eq!(f.value("text"), "已排队");
        assert_eq!(f.binding("n:slider").hwnd, slider);
        assert_eq!(controls::text(f.binding("n:edit").hwnd).unwrap(), "7");
        SendMessageW(
            f.panel.handle(),
            WM_HSCROLL,
            Some(WPARAM(TB_ENDTRACK as usize)),
            Some(LPARAM(slider.0 as isize)),
        );
        f.tick();
        assert_eq!(f.value("n"), 7);
        assert!(f.panel.exit_idle());
    }
}

#[test]
fn retiring_inline_view_preserves_the_tab_owner_expansion() {
    unsafe {
        let (f, _parent) = fixture(None);
        f.state.parameters_ui.expanded.set(true);
        DestroyWindow(f.panel.handle()).unwrap();
        assert!(f.state.parameters_ui.expanded.get());
        assert!(f.state.parameters_ui.window().is_invalid());
    }
}
