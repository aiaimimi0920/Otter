//! 真实隐藏Application：捕获提交→后台历史→缩略图新钉图，失败和普通打开均不入栈。
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
        let instance = HINSTANCE(
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .unwrap()
                .0,
        );
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Native.Workspace"),
                lpfnWndProc: Some(crate::desktop::procedure::window_proc),
                ..Default::default()
            }),
            0
        );
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
impl Drop for Environment {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.instance));
        }
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
        let id = {
            let view = app.capture_history.view.borrow();
            assert_eq!(view.data.screenshots.len(), 1);
            assert_eq!(
                (
                    view.data.screenshots[0].width,
                    view.data.screenshots[0].height
                ),
                (480, 240)
            );
            view.data.screenshots[0].id.clone()
        };
        app.apply_history(&origin, Action::Reuse(id), false)
            .unwrap();
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
        assert!(app
            .add_prepared_with(vec![prepared(true)], false, |_, _, _, _| Err(Error::new(
                E_FAIL, "injected"
            )))
            .is_err());
        assert!(app.capture_history.view.borrow().captures.is_empty());
        app.add_prepared(vec![prepared(false)], false).unwrap();
        assert!(app.capture_history.view.borrow().captures.is_empty());
        let mut origin = crate::desktop::capture_history_ui::origin(&app.documents[0].state);
        origin.lifetime = Default::default();
        app.apply_history(&origin, Action::Reuse("missing".into()), false)
            .unwrap();
        assert_eq!(app.documents.len(), 2);
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
        let id = view.borrow().data.screenshots[0].id.clone();
        for approved in [false, true] {
            app.apply_history_with(
                &origin,
                Action::Delete(id.clone()),
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
