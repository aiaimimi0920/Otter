//! 实际编辑栏 BUTTON→授权意图；缓存真正修改另由执行 owner 的 TLS 回归覆盖。
use super::*;
use crate::{
    desktop::{extension_document, extension_overlay},
    extension_visual as visual,
};
use std::sync::Arc;
unsafe fn setup(f: &Fixture) {
    f.state.extensions.borrow_mut().snapshot = Some(Arc::new(visual::tests::catalog()));
    let raw = serde_json::value::RawValue::from_string("{\"revision\":4}".into()).unwrap();
    *f.state.extension_document.borrow_mut() =
        extension_document::Owner::restore(visual::tests::attachments(), Some(&raw));
    extension_overlay::sync(f.owner.0, &f.state).unwrap();
    refresh(&f.state);
}
unsafe fn click(f: &Fixture, id: u16) {
    SendMessageW(
        GetDlgItem(Some(f.panel.0), id as i32).unwrap(),
        BM_CLICK,
        None,
        None,
    );
    let mut m = MSG::default();
    assert!(PeekMessageW(
        &mut m,
        Some(f.owner.0),
        crate::desktop::tool_panel::EXECUTE,
        crate::desktop::tool_panel::EXECUTE,
        PM_REMOVE
    )
    .as_bool());
    DispatchMessageW(&m);
}
#[test]
fn cached_buttons_enqueue_authorized_work_instead_of_mutating_attachments_in_the_button_callback() {
    unsafe {
        for id in [catalog::OCR, catalog::TRANSLATE] {
            let f = Fixture::new();
            setup(&f);
            let stamp = f.state.extension_document.borrow().store.stamp();
            f.state
                .canvas
                .borrow_mut()
                .select_tool(crate::editor::Tool::Brush);
            click(&f, id);
            assert!(crate::desktop::extension_commands::busy(&f.state));
            assert!(stamp.matches(&f.state.extension_document.borrow().store));
            assert_eq!(f.state.canvas.borrow().tool, crate::editor::Tool::Select);
            assert!(f.state.canvas.borrow().selection_enabled);
            assert!(!IsWindowEnabled(GetDlgItem(Some(f.panel.0), id as i32).unwrap()).as_bool());
        }
    }
}
#[test]
fn stale_busy_actions_are_rejected_and_source_change_invalidates_translation_not_cache() {
    unsafe {
        let f = Fixture::new();
        setup(&f);
        let stamp = f.state.extension_document.borrow().store.stamp();
        f.state.file_dialog.set(true);
        execute(f.owner.0, &f.state, catalog::OCR as usize, false);
        assert!(stamp.matches(&f.state.extension_document.borrow().store));
        f.state.file_dialog.set(false);
        f.state.canvas.borrow_mut().normal_opacity = 0.5;
        extension_overlay::sync(f.owner.0, &f.state).unwrap();
        refresh(&f.state);
        assert_eq!(f.state.extension_document.borrow().revision(), Some(4));
        assert!(
            IsWindowEnabled(GetDlgItem(Some(f.panel.0), catalog::TRANSLATE as i32).unwrap())
                .as_bool()
        );
        {
            let mut c = f.state.canvas.borrow_mut();
            c.editor
                .begin(
                    crate::editor::Tool::Brush,
                    crate::viewport::Point { x: 1., y: 1. },
                )
                .unwrap();
            assert!(c.editor.commit());
        }
        extension_overlay::sync(f.owner.0, &f.state).unwrap();
        refresh(&f.state);
        assert_eq!(f.state.extension_document.borrow().revision(), Some(5));
        assert!(!crate::extension_visual::cached(
            f.state.extensions.borrow().snapshot.as_ref().unwrap(),
            &f.state.extension_document.borrow().store,
            5,
            true
        ));
        assert!(
            IsWindowEnabled(GetDlgItem(Some(f.panel.0), catalog::OCR as i32).unwrap()).as_bool()
        );
        assert!(stamp.matches(&f.state.extension_document.borrow().store));
        assert!(!extension_overlay::sync(f.owner.0, &f.state).unwrap());
    }
}
