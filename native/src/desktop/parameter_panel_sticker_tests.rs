//! Real button notifications from ordinary Tab inline children; no system file dialog or global input.
use super::*;
#[test]
fn ordinary_sticker_has_real_upload_clear_disable_controls_and_uses_same_transaction() {
    unsafe {
        let fixture = Fixture::new();
        fixture.state.selected_art.borrow_mut().take();
        let asset = desktop::sticker_input::tests::asset();
        fixture.state.sticker_input.set_asset(Some(asset)).unwrap();
        fixture.tick();
        for key in ["sticker:upload", "sticker:clear", "sticker:disable"] {
            assert!(IsWindow(Some(fixture.binding(key).hwnd)).as_bool());
        }
        let before = Stamp::capture(&fixture.state);
        fixture.click("sticker:disable");
        fixture.tick();
        assert!(fixture.state.sticker_input.snapshot().disabled);
        assert!(!before.matches(&fixture.state));
        fixture.click("sticker:clear");
        fixture.tick();
        assert!(fixture.state.sticker_input.snapshot().asset.is_none());
        assert!(fixture.state.sticker_input.snapshot().disabled);
        let parent = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("sticker rail"),
            WS_CHILD,
            0,
            0,
            300,
            240,
            Some(fixture.host.0),
            None,
            None,
            None,
        )
        .unwrap();
        let panel = Panel::inline(fixture.host.0, &fixture.state, parent).unwrap();
        assert_eq!(
            GetWindowLongW(panel.handle(), GWL_STYLE) as u32 & WS_CHILD.0,
            WS_CHILD.0
        );
        assert_eq!(panel.ui.model.borrow().rows.len(), 1);
        assert!(panel
            .ui
            .bindings
            .borrow()
            .iter()
            .any(|binding| binding.key == "image:upload"));
        assert!(panel
            .ui
            .model
            .borrow()
            .rows
            .iter()
            .all(|row| row.id != "output_image"));
        drop(panel);
        DestroyWindow(parent).unwrap();
    }
}
#[test]
fn retired_locked_hidden_and_old_configuration_actions_are_refused() {
    unsafe {
        let fixture = Fixture::new();
        fixture.state.selected_art.borrow_mut().take();
        fixture.tick();
        use desktop::sticker_input::{self, Action};
        for gate in 0..4 {
            let before = Stamp::capture(&fixture.state);
            if gate == 0 {
                fixture.state.group_locked.set(true);
            }
            if gate == 1 {
                fixture.state.group_hidden.set(true);
            }
            if gate == 2 {
                fixture.state.sticker_input.toggle().unwrap();
            }
            assert_eq!(
                sticker_input::apply(
                    fixture.host.0,
                    &fixture.state,
                    &before,
                    Action::Clear,
                    &|| gate != 3
                )
                .unwrap(),
                desktop::parameter_file_input::Outcome::Rejected
            );
            fixture.state.group_locked.set(false);
            fixture.state.group_hidden.set(false);
        }
    }
}

#[test]
fn cancelled_image_choice_clears_error_without_commit_or_configuration_change() {
    unsafe {
        let fixture = Fixture::new();
        fixture.state.selected_art.borrow_mut().take();
        fixture.tick();
        let before = Stamp::capture(&fixture.state);
        fixture.panel.ui.error("旧错误");
        fixture
            .panel
            .sticker_outcome(desktop::parameter_file_input::Outcome::Cancelled);
        assert!(fixture.panel.ui.message.borrow().is_empty());
        assert!(before.matches(&fixture.state));
        assert!(fixture.state.parameter_file_request.borrow().is_none());
        assert!(fixture.state.sticker_input.snapshot().asset.is_none());
    }
}

#[test]
fn rejected_image_choice_stays_an_error_and_accepted_outcome_clears_it() {
    unsafe {
        use desktop::parameter_file_input::Outcome;
        let fixture = Fixture::new();
        fixture.state.selected_art.borrow_mut().take();
        fixture.tick();
        let before = Stamp::capture(&fixture.state);
        fixture.panel.sticker_outcome(Outcome::Rejected);
        assert_eq!(
            &*fixture.panel.ui.message.borrow(),
            "图片输入配置已变化或暂不可用"
        );
        fixture.panel.sticker_outcome(Outcome::Accepted);
        assert!(fixture.panel.ui.message.borrow().is_empty());
        assert!(before.matches(&fixture.state));
        fixture.state.sticker_input.toggle().unwrap();
        fixture.panel.sticker_outcome(Outcome::Accepted);
        assert!(!before.matches(&fixture.state));
    }
}

#[test]
fn upload_button_erase_keeps_previous_pixels_and_tab_placement_finishes_the_control_frame() {
    unsafe {
        let fixture = Fixture::new();
        fixture.state.selected_art.borrow_mut().take();
        let parent = desktop::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                None,
                WS_POPUP,
                0,
                0,
                256,
                160,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let panel = Panel::inline(fixture.host.0, &fixture.state, parent.0).unwrap();
        let button = panel
            .ui
            .bindings
            .borrow()
            .iter()
            .find(|b| b.key == "image:upload")
            .unwrap()
            .hwnd;
        let dc = CreateCompatibleDC(None);
        let bitmap = CreateBitmap(4, 4, 1, 32, None);
        let old = SelectObject(dc, HGDIOBJ(bitmap.0));
        theme::fill(
            dc,
            &RECT {
                right: 4,
                bottom: 4,
                ..Default::default()
            },
            0x563412,
        );
        assert_eq!(
            SendMessageW(button, WM_ERASEBKGND, Some(WPARAM(dc.0 as usize)), None),
            LRESULT(1)
        );
        assert_eq!(GetPixel(dc, 1, 1), COLORREF(0x563412));
        SelectObject(dc, old);
        DeleteObject(HGDIOBJ(bitmap.0)).unwrap();
        DeleteDC(dc).ok().unwrap();
        let _ = ShowWindow(parent.0, SW_SHOWNOACTIVATE);
        let binding = panel.inline_binding().unwrap();
        let bounds = RECT {
            right: 256,
            bottom: 160,
            ..Default::default()
        };
        let row = RECT {
            right: 228,
            bottom: 28,
            ..Default::default()
        };
        for _ in 0..12 {
            binding.place(bounds, Vec::new()).unwrap();
            // The production owner supplies stable model field names; tests do not guess them.
            let id = panel.ui.model.borrow().rows[0].id.clone();
            binding.place(bounds, vec![(id, row, row)]).unwrap();
            assert!(IsWindowVisible(button).as_bool());
            assert!(!GetUpdateRect(button, None, false).as_bool());
        }
    }
}
