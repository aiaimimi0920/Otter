//! Real hidden Application: capture commit -> background history -> new pin from thumbnail; failures and ordinary opens never enter history.
use super::*;
struct Environment {
    instance: HINSTANCE,
    _com: crate::desktop::Apartment,
    _serial: std::sync::MutexGuard<'static, ()>,
}
impl Environment {
    unsafe fn new() -> Self {
        let serial = crate::loom_bridge_client::tests::execution_job::SERIAL
            .lock()
            .unwrap();
        let com = crate::desktop::Apartment::new().unwrap();
        let instance = crate::desktop::workspace_window_fixture::register();
        Self {
            instance,
            _com: com,
            _serial: serial,
        }
    }
    unsafe fn app(&self) -> Application {
        let initial =
            DocumentWindow::create(self.instance, State::new(None, false).unwrap(), false).unwrap();
        Application::new(self.instance, initial)
    }
}
fn prepared(captured: bool) -> crate::desktop::open_image::PreparedDocument {
    let image = crate::image_decode::Image {
        width: 480,
        height: 240,
        pixels: [10, 20, 30, 255].repeat(480 * 240).into(),
    };
    let mut prepared =
        crate::desktop::open_image::PreparedDocument::from_image(image, true).unwrap();
    prepared.record_history = captured;
    if captured {
        let metadata =
            crate::capture_job::Context::region(None, 480, 240, "gdi").metadata(false, false);
        prepared.capture_source = Some(
            crate::desktop::canvas::CaptureSource::from_session(&metadata.to_string()).unwrap(),
        );
    }
    prepared
}
#[test]
fn committed_capture_records_original_size_and_reuse_creates_only_240_thumbnail_without_reentering_history(
) {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let origin = crate::desktop::capture_history_ui::origin(&app.documents[0].state);
        app.add_prepared(vec![prepared(true)], false).unwrap();
        app.capture_history.shutdown();
        {
            let view = app.capture_history.view.borrow();
            assert_eq!(view.data.screenshots.len(), 1);
            assert!(
                view.data.screenshots[0].preview.is_none(),
                "idle history keeps only compact PNG data URLs until the panel opens"
            );
            assert_eq!(
                (
                    view.data.screenshots[0].width,
                    view.data.screenshots[0].height
                ),
                (480, 240)
            );
        }
        app.capture_history.view.borrow_mut().toggle = Some(origin.clone());
        app.process_capture_history(false);
        assert!(app.history_window.is_some());
        let snapshot = app.capture_history.view.borrow().data.clone();
        let entry = &snapshot.screenshots[0];
        assert!(app
            .capture_history
            .view
            .borrow()
            .previews
            .get(entry)
            .is_none());
        app.capture_history.shutdown(); // Settle the panel's background slot; do not decode on the UI thread.
        let preview = app
            .capture_history
            .view
            .borrow()
            .previews
            .get(entry)
            .unwrap()
            .clone();
        let click = Action::Reuse(entry.key.clone(), preview.image.clone());
        drop(preview);
        app.history_window.take();
        assert!(app
            .capture_history
            .view
            .borrow()
            .previews
            .get(entry)
            .is_none());
        assert!(
            entry.preview.is_none(),
            "a retained save snapshot never pins decoded buffers"
        );
        app.apply_history(&origin, click, false).unwrap();
        assert_eq!(app.documents.len(), 3);
        let last = app.documents.last().unwrap().state.canvas.borrow();
        let image = last.image.as_ref().unwrap();
        assert_eq!((image.width, image.height), (240, 120));
        assert_eq!(&image.pixels[..4], &[10, 20, 30, 255]);
        assert!(last.capture_source().is_none());
        drop(last);
        app.capture_history.shutdown();
        assert_eq!(app.capture_history.view.borrow().data.screenshots.len(), 1);
        app.shutdown();
    }
}
#[test]
fn failed_staging_ordinary_import_and_stale_origin_never_add_history_or_windows() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let last =
            crate::capture_last_region::Last::new([1, 2, 4, 5], vec![[0, 0, 100, 100]]).unwrap();
        let mut capture = prepared(true);
        capture.ordinary_region = Some(last.clone());
        capture.ordinary_capture = true;
        assert!(app
            .add_prepared_with(vec![capture], false, |_, _, _, _| Err(Error::new(
                E_FAIL, "injected"
            )))
            .is_err());
        assert!(app.capture_history.view.borrow().captures.is_empty());
        assert!(app.capture_history.view.borrow().last_region.is_none());
        app.add_prepared(vec![prepared(false)], false).unwrap();
        assert!(app.capture_history.view.borrow().last_region.is_none());
        assert!(app.capture_history.view.borrow().captures.is_empty());
        let mut origin = crate::desktop::capture_history_ui::origin(&app.documents[0].state);
        origin.lifetime = Default::default();
        app.apply_history(
            &origin,
            Action::Reuse(Default::default(), prepared(false).image),
            false,
        )
        .unwrap();
        assert_eq!(app.documents.len(), 2);
        app.shutdown();
    }
}
#[test]
fn visible_capture_starts_in_free_mode_without_opening_the_edit_strip() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        app.documents[0]
            .state
            .canvas
            .borrow_mut()
            .select_tool(crate::editor::Tool::Brush);
        app.add_prepared(vec![prepared(true)], true).unwrap();
        let doc = app.documents.last().unwrap();
        assert!(IsWindowVisible(doc.window.0).as_bool());
        assert!(!IsWindowVisible(doc.panel.0).as_bool());
        assert_eq!(doc.state.canvas.borrow().tool, crate::editor::Tool::Pan);
        assert!(doc.state.canvas.borrow().selected_indices().is_empty());
        assert!(neuro_mini_ui::property_window(doc.panel.0).is_none());
        app.add_prepared(vec![prepared(false)], true).unwrap();
        assert!(IsWindowVisible(app.documents.last().unwrap().panel.0).as_bool());
        assert!(!IsWindowVisible(app.documents[1].panel.0).as_bool());
        app.shutdown();
    }
}
#[test]
fn color_reuse_is_nonfatal_on_copy_failure_and_delete_requires_confirmation_of_same_entry() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let origin = crate::desktop::capture_history_ui::origin(&app.documents[0].state);
        let color = crate::stroke_style::Color(1, 2, 3);
        let result = app.apply_history_with(
            &origin,
            Action::Color(color, "#010203".into()),
            false,
            |_, hex, current| {
                assert_eq!(hex, "#010203");
                assert!(current());
                Err(Error::new(E_FAIL, "injected clipboard failure"))
            },
            |_| panic!("color must not confirm"),
        );
        assert!(result.is_err());
        assert_eq!(
            app.documents[0]
                .state
                .canvas
                .borrow()
                .editor
                .style()
                .color(),
            color
        );
        app.add_prepared(vec![prepared(true)], false).unwrap();
        app.capture_history.shutdown();
        let view = app.capture_history.view.clone();
        let original_key = view.borrow().data.screenshots[0].key.clone();
        app.apply_history_with(
            &origin,
            Action::Delete(original_key.clone()),
            false,
            |_, _, _| unreachable!(),
            |_| {
                // Even identical ID, timestamp and encoded pixels identify a different capture.
                let mut replacement = view.borrow().data.screenshots[0].clone();
                replacement.key = Default::default();
                view.borrow_mut().screenshot(replacement);
                true
            },
        )
        .unwrap();
        let key = view.borrow().data.screenshots[0].key.clone();
        app.apply_history_with(
            &origin,
            Action::Delete(original_key),
            false,
            |_, _, _| unreachable!(),
            |_| panic!("must not confirm a click queued before replacement"),
        )
        .unwrap();
        for approved in [false, true] {
            app.apply_history_with(
                &origin,
                Action::Delete(key.clone()),
                false,
                |_, _, _| panic!("delete must not copy"),
                |_| approved,
            )
            .unwrap();
            assert_eq!(
                view.borrow().data.screenshots.len(),
                if approved { 0 } else { 1 }
            );
            assert_eq!(app.documents.len(), 2);
            assert_eq!(
                app.documents[1]
                    .state
                    .canvas
                    .borrow()
                    .image
                    .as_ref()
                    .unwrap()
                    .width,
                480
            );
        }
        app.shutdown();
    }
}
