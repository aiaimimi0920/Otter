//! 普通Tab及单字段child的真实按钮通知；不打开系统文件框或发送全局输入。
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
        let panel = Panel::embedded(fixture.host.0, &fixture.state, parent, "image").unwrap();
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
            .any(|binding| binding.key == "sticker:upload"));
        assert!(Panel::embedded(fixture.host.0, &fixture.state, parent, "output_image").is_err());
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
        fixture.panel.sticker_outcome(
            &fixture.state,
            &before,
            desktop::parameter_file_input::Outcome::Cancelled,
        );
        assert!(fixture.panel.ui.message.borrow().is_empty());
        assert!(!fixture.panel.ui.committed.get());
        assert!(before.matches(&fixture.state));
        assert!(fixture.state.parameter_file_request.borrow().is_none());
        assert!(fixture.state.sticker_input.snapshot().asset.is_none());
    }
}

#[test]
fn rejected_image_choice_stays_an_error_and_only_actual_change_marks_committed() {
    unsafe {
        use desktop::parameter_file_input::Outcome;
        let fixture = Fixture::new();
        fixture.state.selected_art.borrow_mut().take();
        fixture.tick();
        let before = Stamp::capture(&fixture.state);
        fixture
            .panel
            .sticker_outcome(&fixture.state, &before, Outcome::Rejected);
        assert_eq!(
            &*fixture.panel.ui.message.borrow(),
            "图片输入配置已变化或暂不可用"
        );
        assert!(!fixture.panel.ui.committed.get());
        fixture
            .panel
            .sticker_outcome(&fixture.state, &before, Outcome::Accepted);
        assert!(fixture.panel.ui.message.borrow().is_empty());
        assert!(!fixture.panel.ui.committed.get());
        fixture.state.sticker_input.toggle().unwrap();
        fixture
            .panel
            .sticker_outcome(&fixture.state, &before, Outcome::Accepted);
        assert!(fixture.panel.ui.committed.get());
    }
}
