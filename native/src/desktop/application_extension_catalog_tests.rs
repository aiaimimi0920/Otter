//! 实际 Application 的扩展目录共享与关闭撤权；不启动发现、托盘、全局输入或用户实例。
use super::*;
use std::{rc::Rc, sync::Arc};
struct Class {
    instance: HINSTANCE,
    owned: bool,
}
impl Class {
    unsafe fn new() -> Self {
        let instance = HINSTANCE(
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .unwrap()
                .0,
        );
        let owned = RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("Otter.Native.Workspace"),
            lpfnWndProc: Some(crate::desktop::procedure::window_proc),
            ..Default::default()
        }) != 0;
        assert!(owned || GetLastError() == ERROR_CLASS_ALREADY_EXISTS);
        Self { instance, owned }
    }
}
impl Drop for Class {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.instance)).unwrap();
            }
        }
    }
}
fn state() -> State {
    State::new(
        Some(crate::image_decode::Image {
            width: 1,
            height: 1,
            pixels: vec![255; 4].into(),
        }),
        true,
    )
    .unwrap()
}
#[test]
fn new_and_existing_documents_share_one_catalog_and_shutdown_revokes_all_views() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let class = Class::new();
        let state = state();
        let view = state.extensions.clone();
        let initial = DocumentWindow::create(class.instance, state, false).unwrap();
        let mut app = Application::new(class.instance, initial);
        assert!(Rc::ptr_eq(&view, &app.extensions.view));
        let next = app.prepare_window(self::state(), false).unwrap();
        assert!(Rc::ptr_eq(&next.state.extensions, &view));
        let source = crate::loom_extension_snapshot::tests::fixture();
        let snapshot = Arc::new(
            crate::loom_extension_snapshot::Snapshot::parse(&serde_json::to_vec(&source).unwrap())
                .unwrap(),
        );
        app.extensions.view.borrow_mut().snapshot = Some(snapshot.clone());
        assert!(Arc::ptr_eq(
            view.borrow().snapshot.as_ref().unwrap(),
            &snapshot
        ));
        assert!(Arc::ptr_eq(
            next.state.extensions.borrow().snapshot.as_ref().unwrap(),
            &snapshot
        ));
        app.extensions.shutdown();
        assert!(view.borrow().snapshot.is_none());
        assert!(next.state.extensions.borrow().snapshot.is_none());
    }
}
