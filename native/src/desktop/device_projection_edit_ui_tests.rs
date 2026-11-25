//! 纯元数据不借用编辑器；菜单和命令仍核动态编辑视图及同 origin 准入。
use super::*;
use crate::projection_edit::{
    self as edit, fixture,
    journal::Context,
    view::{Size, View as EditView},
    workspace::{Role, Snapshot},
};
use serde_json::json;

fn state(role: Role, mode: edit::Mode, conflict: bool) -> State {
    let state = State::new(
        Some(crate::desktop::image::Image {
            width: 32,
            height: 16,
            pixels: vec![255; 32 * 16 * 4].into(),
        }),
        false,
    )
    .unwrap();
    let mut raw = fixture::document_value();
    raw["objects"] = json!({});
    raw["mode"] = json!(if mode == edit::Mode::OneWay {
        "one_way"
    } else {
        "two_way"
    });
    let document = edit::Document::parse(&serde_json::to_vec(&raw).unwrap()).unwrap();
    let view = EditView::from_document(&document, Size::new(32.0, 16.0).unwrap()).unwrap();
    let context = Context::new(
        serde_json::from_value(json!("a".repeat(64))).unwrap(),
        state.unit_id.as_str(),
        "https://loom.example",
        "device-1",
    )
    .unwrap();
    let snapshot = Snapshot::new(
        context,
        fixture::PROJECTION,
        edit::Route::Shared,
        role,
        document,
        &view,
        conflict.then_some(edit::Code::ObjectConflict),
    )
    .unwrap();
    {
        let mut canvas = state.canvas.borrow_mut();
        let (editor, owner) = crate::desktop::projection_edit_workspace::Owner::restore(
            snapshot,
            std::mem::take(&mut canvas.editor),
        )
        .unwrap();
        canvas.editor = editor;
        *state.projection_edit.borrow_mut() = Some(owner);
    }
    state.authorization.borrow_mut().origin = Some("https://loom.example".into());
    state
}
#[test]
fn authoritative_mode_and_role_do_not_capture_or_borrow_editor() {
    for role in [Role::Source, Role::Receiver] {
        for mode in [edit::Mode::OneWay, edit::Mode::TwoWay] {
            let state = state(role, mode, true);
            // capture 必须借 canvas；持有独占借用直接保护无 capture 的读取契约。
            let _canvas = state.canvas.borrow_mut();
            assert_eq!(editing::mode(&state), mode);
            assert_eq!(editing::can_choose_mode(&state), role == Role::Source);
            state.authorization.borrow_mut().closed = true;
            assert!(!editing::can_choose_mode(&state));
        }
    }
}
unsafe fn enabled(state: &State, id: usize) -> bool {
    let menu = CreatePopupMenu().unwrap();
    editing::append_menu(menu, state, &state.authorization.borrow()).unwrap();
    let flags = GetMenuState(menu, id as u32, MF_BYCOMMAND);
    DestroyMenu(menu).unwrap();
    assert_ne!(flags, u32::MAX, "missing command {id}");
    flags & MF_GRAYED.0 == 0
}
#[test]
fn menu_and_command_recheck_draft_unsupported_content_basis_and_origin() {
    unsafe {
        let window = window();
        for case in 0..4 {
            let state = state(Role::Source, edit::Mode::OneWay, true);
            let commands = [
                editing::SUBMIT,
                editing::TWO_WAY,
                editing::KEEP_LOCAL,
                editing::USE_REMOTE,
            ];
            for command in commands {
                assert!(enabled(&state, command));
            }
            match case {
                0 | 1 => {
                    state
                        .canvas
                        .borrow_mut()
                        .editor
                        .begin(
                            crate::editor::Tool::Mosaic,
                            crate::viewport::Point { x: 12.0, y: 8.0 },
                        )
                        .unwrap();
                    if case == 1 {
                        assert!(state.canvas.borrow_mut().editor.commit());
                    }
                }
                2 => {
                    let mut canvas = state.canvas.borrow_mut();
                    let copy = canvas.editor.layers().unwrap().base.to_vec();
                    assert!(canvas.editor.replace_source(copy.into()));
                }
                _ => state.authorization.borrow_mut().origin = Some("https://other.example".into()),
            }
            for command in commands {
                assert!(!enabled(&state, command), "case {case}, command {command}");
                assert!(editing::execute(window.0, &state, command).is_err());
            }
            assert!(state.authorization.borrow().request.is_none());
            assert_eq!(editing::mode(&state), edit::Mode::OneWay);
            if case == 0 {
                assert!(state.canvas.borrow_mut().editor.cancel());
                assert!(enabled(&state, editing::SUBMIT));
            }
        }
    }
}
#[test]
fn receiver_direction_and_conflict_permissions_keep_authoritative_mode() {
    unsafe {
        for mode in [edit::Mode::OneWay, edit::Mode::TwoWay] {
            let window = window();
            let state = state(Role::Receiver, mode, true);
            assert!(!enabled(&state, editing::ONE_WAY));
            assert!(!enabled(&state, editing::TWO_WAY));
            assert!(enabled(&state, editing::USE_REMOTE));
            assert_eq!(
                enabled(&state, editing::KEEP_LOCAL),
                mode == edit::Mode::TwoWay
            );
            assert!(editing::execute(window.0, &state, editing::TWO_WAY).is_err());
            if mode == edit::Mode::OneWay {
                assert!(editing::execute(window.0, &state, editing::KEEP_LOCAL).is_err());
            }
            assert!(state.authorization.borrow().request.is_none());
        }
    }
}
#[test]
fn source_mode_selection_keeps_exact_identity_and_absent_conflict_is_rejected() {
    unsafe {
        let window = window();
        let state = state(Role::Source, edit::Mode::OneWay, false);
        for command in [editing::ONE_WAY, editing::KEEP_LOCAL, editing::USE_REMOTE] {
            assert!(editing::execute(window.0, &state, command).is_err());
        }
        let expected = state.projection_edit.borrow().as_ref().unwrap().stamp();
        editing::execute(window.0, &state, editing::TWO_WAY).unwrap();
        match state.authorization.borrow_mut().request.take().unwrap() {
            Request::SetProjectionEditMode(selection, mode) => {
                assert_eq!(selection.unit, state.unit_id);
                assert!(selection.stamp == expected);
                assert_eq!(mode, edit::Mode::TwoWay);
            }
            _ => panic!("expected mode request"),
        };
    }
}
