//! 注入剪贴板序号和最终发布，真实 worker/文件/隐藏 HWND 共用生产生命周期。
use super::*;
fn wait_ready(state: &State) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !state
        .copy_job
        .borrow()
        .as_ref()
        .unwrap()
        .worker
        .as_ref()
        .unwrap()
        .is_finished()
    {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
#[test]
fn newer_window_copy_cancels_old_completed_stage_and_can_acquire_same_cache_lock() {
    unsafe {
        let first = Fixture::new();
        let second = Fixture::new();
        let old = state();
        let mut latest = state();
        latest.copy_authority = old.copy_authority.clone();
        start_with_plan(first.hwnd, &old, Some(first.plan()));
        wait_ready(&old);
        let old_cancel = old.copy_job.borrow().as_ref().unwrap().cancel.clone();
        start_with_plan(second.hwnd, &latest, Some(first.plan()));
        assert!(old_cancel.load(Ordering::Acquire));
        assert_eq!(finish(&first, &old), 0);
        assert_eq!(finish(&second, &latest), 1);
        assert_eq!(first.files().len(), 1);
        assert_eq!(first.files()[0].file_name().unwrap(), "复制🙂.png");
    }
}
#[test]
fn cancelling_latest_request_never_revives_the_previous_window_copy() {
    unsafe {
        let first = Fixture::new();
        let second = Fixture::new();
        let old = state();
        let mut latest = state();
        latest.copy_authority = old.copy_authority.clone();
        start_with_plan(first.hwnd, &old, Some(first.plan()));
        wait_ready(&old);
        start_with_plan(second.hwnd, &latest, None);
        cancel(&latest);
        assert_eq!(finish(&first, &old), 0);
        assert_eq!(finish(&second, &latest), 0);
        assert!(first.files().is_empty());
    }
}
#[test]
fn external_sequence_change_drops_stage_before_file_and_clipboard_publication() {
    unsafe {
        let fixture = Fixture::new();
        let state = state();
        start_guarded(fixture.hwnd, &state, Some(fixture.plan()), Some(41));
        wait_ready(&state);
        poll_guarded(
            fixture.hwnd,
            &state,
            || 42,
            |_, _, _, _| panic!("newer external clipboard must survive"),
        );
        assert!(state.copy_job.borrow().is_none());
        assert!(fixture.files().is_empty());
        start_guarded(fixture.hwnd, &state, Some(fixture.plan()), Some(42));
        wait_ready(&state);
        let mut count = 0;
        poll_guarded(
            fixture.hwnd,
            &state,
            || 42,
            |_, payload, paths, expected| {
                assert_eq!(expected, Some(42));
                assert_eq!(paths.len(), 1);
                assert_eq!(fs::read(&paths[0]).unwrap(), &*payload.png);
                count += 1;
                Ok(Some(43))
            },
        );
        assert_eq!(count, 1);
        assert_eq!(fixture.files().len(), 1);
    }
}

#[test]
fn editable_cache_changes_only_after_confirmed_publication_and_rejects_stale_sequence() {
    unsafe {
        let fixture = Fixture::new();
        let state = state();
        for (before, published) in [(41, Some(42)), (42, None), (42, Some(0)), (44, Some(45))] {
            start_guarded(fixture.hwnd, &state, None, Some(before));
            wait_ready(&state);
            poll_guarded(fixture.hwnd, &state, || before, |_, _, _, _| Ok(published));
            let expected = published.unwrap_or(before);
            assert_eq!(
                state
                    .editable_clipboard
                    .borrow_mut()
                    .matching(expected)
                    .is_some(),
                expected != 0
            );
        }
        assert!(state.editable_clipboard.borrow_mut().matching(46).is_none());
        assert!(state.editable_clipboard.borrow_mut().matching(45).is_none());
        start_guarded(fixture.hwnd, &state, None, Some(46));
        wait_ready(&state);
        cancel(&state);
        poll_guarded(
            fixture.hwnd,
            &state,
            || 46,
            |_, _, _, _| panic!("cancelled publication"),
        );
        assert!(state.editable_clipboard.borrow_mut().matching(46).is_none());
    }
}

#[test]
fn displayed_copy_pastes_static_edits_without_target_capture_or_live_owner() {
    unsafe {
        let fixture = Fixture::new();
        let mut source = state();
        let mut target = state();
        target.editable_clipboard = source.editable_clipboard.clone();
        let image = source.canvas.borrow().image.clone().unwrap();
        let mut overlay = crate::editor::Editor::default();
        overlay
            .begin(
                crate::editor::Tool::Brush,
                crate::viewport::Point { x: 1.0, y: 1.0 },
            )
            .unwrap();
        overlay.commit();
        *source.resolved_display.borrow_mut() =
            Some(crate::desktop::display_frame::Frame::capture(
                crate::desktop::document_state::new_unit_id().unwrap(),
                image.clone(),
                Some(&overlay),
            ));
        source.canvas.borrow_mut().attach_capture(Some(
            crate::desktop::canvas::CaptureSource::from_session(
                r#"{"kind":"region","futureClipboard":{"wrongTarget":true}}"#,
            )
            .unwrap(),
        ));
        start_guarded(fixture.hwnd, &source, None, Some(70));
        wait_ready(&source);
        poll_guarded(fixture.hwnd, &source, || 70, |_, _, _, _| Ok(Some(71)));
        let content = target.editable_clipboard.borrow_mut().matching(71).unwrap();
        assert!(content.capture_source.is_none());
        crate::desktop::paste_input::deliver(
            fixture.hwnd,
            &target,
            crate::desktop::paste_input::Snapshot::Editable(content),
            true,
        )
        .unwrap();
        crate::desktop::open_job::finish(fixture.hwnd, &target, true);
        let prepared = target.new_pin.borrow_mut().take().unwrap().pop().unwrap();
        let pasted = State::from_prepared(prepared).unwrap();
        let mut canvas = pasted.canvas.borrow_mut();
        assert!(canvas
            .image
            .as_ref()
            .unwrap()
            .pixels
            .same_storage(&image.pixels));
        assert_eq!(canvas.editor.elements().len(), 1);
        assert_ne!(
            canvas.editor.elements()[0].annotation_id(),
            overlay.elements()[0].annotation_id()
        );
        assert!(!canvas.editor.undo());
        assert!(canvas.capture_source().is_none());
        assert!(pasted.resolved_display.borrow().is_none());
        assert!(pasted.live_request.borrow().is_none());
        assert!(!pasted.is_art());
        // Art remains flattened even when a resolved frame carries editable overlay data.
        source.restored_art = true;
        start_guarded(fixture.hwnd, &source, None, Some(71));
        wait_ready(&source);
        poll_guarded(fixture.hwnd, &source, || 71, |_, _, _, _| Ok(Some(72)));
        assert!(target
            .editable_clipboard
            .borrow_mut()
            .matching(72)
            .is_none());
    }
}
#[test]
fn confirmed_copy_is_pasted_from_another_window_with_new_editable_identity() {
    unsafe {
        let fixture = Fixture::new();
        let mut source = state();
        let mut target = state();
        target.editable_clipboard = source.editable_clipboard.clone();
        source
            .canvas
            .borrow_mut()
            .editor
            .begin(
                crate::editor::Tool::Brush,
                crate::viewport::Point { x: 1.0, y: 1.0 },
            )
            .unwrap();
        source.canvas.borrow_mut().editor.commit();
        source.canvas.borrow_mut().attach_capture(Some(crate::desktop::canvas::CaptureSource::from_session(
            r#"{"kind":"region","dynamicRange":"sdr","captureBackend":"gdi","futureClipboard":{"keep":true}}"#,
        ).unwrap()));
        let stamp = source.canvas.borrow().editor.document_stamp();
        start_guarded(fixture.hwnd, &source, None, Some(50));
        wait_ready(&source);
        poll_guarded(fixture.hwnd, &source, || 50, |_, _, _, _| Ok(Some(51)));
        let content = target.editable_clipboard.borrow_mut().matching(51).unwrap();
        crate::desktop::paste_input::deliver(
            fixture.hwnd,
            &target,
            crate::desktop::paste_input::Snapshot::Editable(content),
            true,
        )
        .unwrap();
        crate::desktop::open_job::finish(fixture.hwnd, &target, true);
        let mut prepared = target.new_pin.borrow_mut().take().unwrap();
        let pasted = State::from_prepared(prepared.pop().unwrap()).unwrap();
        assert_ne!(pasted.unit_id, source.unit_id);
        assert_eq!(pasted.canvas.borrow().editor.elements().len(), 1);
        assert!(!stamp.same_document(&pasted.canvas.borrow().editor));
        assert!(stamp.matches(&source.canvas.borrow().editor));
        assert_eq!(
            pasted.canvas.borrow().capture_source().unwrap().metadata(),
            source.canvas.borrow().capture_source().unwrap().metadata()
        );
        assert_ne!(
            source.canvas.borrow().editor.elements()[0].annotation_id(),
            pasted.canvas.borrow().editor.elements()[0].annotation_id()
        );
        // A successful non-editable copy must retire a previous editable receipt.
        source.restored_art = true;
        start_guarded(fixture.hwnd, &source, None, Some(51));
        wait_ready(&source);
        poll_guarded(fixture.hwnd, &source, || 51, |_, _, _, _| Ok(Some(52)));
        assert!(target
            .editable_clipboard
            .borrow_mut()
            .matching(52)
            .is_none());
    }
}
