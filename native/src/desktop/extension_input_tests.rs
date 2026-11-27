//! Real hidden-pin messages/HRGNs and injectable copy; isolated desktops only, without user-clipboard I/O.
use super::*;
use crate::{
    desktop::{document_window::DocumentWindow, extension_document},
    unit_attachment_store::tests as store,
};
use serde_json::json;
use std::sync::Arc;
struct Fixture {
    document: DocumentWindow,
    _apartment: crate::desktop::Apartment,
}
impl Fixture {
    unsafe fn new() -> Self {
        let apartment = crate::desktop::Apartment::new().unwrap();
        let instance = crate::desktop::workspace_window_fixture::register();
        let state = State::new(
            Some(crate::image_decode::Image {
                width: 240,
                height: 140,
                pixels: vec![0; 240 * 140 * 4].into(),
            }),
            true,
        )
        .unwrap();
        let mut catalog = crate::extension_visual::tests::catalog_value();
        catalog["contributions"]["renderers"][0]["payload"]["payload"]["bounds"] =
            json!({"x":10,"y":12,"width":200,"height":90});
        state.extensions.borrow_mut().snapshot = Some(Arc::new(
            crate::loom_extension_snapshot::Snapshot::parse(&serde_json::to_vec(&catalog).unwrap())
                .unwrap(),
        ));
        let mut a = store::attachment();
        a["payload"]["surfaceScene"] = json!({"id":"root","type":"stack","layout":{"width":200,"height":90},"children":[
            {"id":"text","type":"text","props":{"text":"Native overlay","selectable":true},"layout":{"width":160,"height":30}}]});
        a["overlayVisible"] = true.into();
        a["payloadDigest"] = crate::unit_attachment_json::digest(&a["payload"]).into();
        *state.extension_document.borrow_mut() =
            extension_document::Owner::restore(store::store(vec![a]), None);
        let document = DocumentWindow::create(instance, state, false).unwrap();
        super::super::extension_overlay::sync(document.window.0, &document.state).unwrap();
        Self {
            document,
            _apartment: apartment,
        }
    }
    fn state(&self) -> &State {
        &self.document.state
    }
    fn hwnd(&self) -> HWND {
        self.document.window.0
    }
    fn view(&self) -> Rc<Views> {
        views(self.state()).unwrap()
    }
    fn point(&self) -> [f32; 2] {
        let v = self.view();
        let r = v.rects()[0];
        [r.left + 0.1, r.top + 2.]
    }
    unsafe fn lp(&self, p: [f32; 2]) -> LPARAM {
        let f = GetDpiForWindow(self.hwnd()).max(96) as f32 / 96.;
        LPARAM(
            (((p[0] * f).round() as i32) as u16 as isize)
                | ((((p[1] * f).round() as i32) as u16 as isize) << 16),
        )
    }
    unsafe fn send(&self, msg: u32, p: [f32; 2]) {
        SendMessageW(self.hwnd(), msg, None, Some(self.lp(p)));
    }
}
#[test]
fn real_pin_text_drag_owns_capture_and_selection_without_moving_or_creating_annotations() {
    unsafe {
        let f = Fixture::new();
        let p = f.point();
        let mut frame = RECT::default();
        GetWindowRect(f.hwnd(), &mut frame).unwrap();
        let stamp = f.state().canvas.borrow().editor.document_stamp();
        f.send(WM_LBUTTONDOWN, p);
        assert_eq!(GetCapture(), f.hwnd());
        let end = [205., p[1]];
        f.send(WM_MOUSEMOVE, end);
        f.send(WM_LBUTTONUP, end);
        assert!(GetCapture().is_invalid());
        assert_eq!(
            f.view().selected_text().unwrap().as_deref(),
            Some("Native overlay")
        );
        let mut after = RECT::default();
        GetWindowRect(f.hwnd(), &mut after).unwrap();
        assert_eq!(frame, after);
        assert!(stamp.matches(&f.state().canvas.borrow().editor));
        assert!(!f.state().canvas.borrow().blocks_wheel());
        assert!(f.state().error.borrow().is_none());
    }
}
#[test]
fn overlay_text_on_transparent_png_is_client_input_but_background_still_drags_the_pin() {
    unsafe {
        let f = Fixture::new();
        let p = f.point();
        let hit = |p: [f32; 2]| {
            let mut p = POINT {
                x: (p[0] * GetDpiForWindow(f.hwnd()) as f32 / 96.).round() as i32,
                y: (p[1] * GetDpiForWindow(f.hwnd()) as f32 / 96.).round() as i32,
            };
            assert!(windows::Win32::Graphics::Gdi::ClientToScreen(f.hwnd(), &mut p).as_bool());
            SendMessageW(
                f.hwnd(),
                WM_NCHITTEST,
                None,
                Some(LPARAM(p.x as u16 as isize | ((p.y as u16 as isize) << 16))),
            )
            .0
        };
        assert_eq!(hit(p), HTCLIENT as isize);
        assert_eq!(hit([220., 120.]), HTCAPTION as isize);
        let region = windows::Win32::Graphics::Gdi::CreateRectRgn(0, 0, 0, 0);
        assert_ne!(
            windows::Win32::Graphics::Gdi::GetWindowRgn(f.hwnd(), region),
            windows::Win32::Graphics::Gdi::RGN_ERROR
        );
        let factor = GetDpiForWindow(f.hwnd()).max(96) as f32 / 96.;
        assert!(windows::Win32::Graphics::Gdi::PtInRegion(
            region,
            (p[0] * factor).round() as i32,
            (p[1] * factor).round() as i32
        )
        .as_bool());
        assert!(!windows::Win32::Graphics::Gdi::PtInRegion(
            region,
            (220. * factor) as i32,
            (120. * factor) as i32
        )
        .as_bool());
        let _ = windows::Win32::Graphics::Gdi::DeleteObject(region.into());
    }
}
#[test]
fn painting_modes_and_minified_images_never_take_overlay_input_and_revoke_releases_capture() {
    unsafe {
        let f = Fixture::new();
        let p = f.point();
        let _ = ShowWindow(f.document.panel.0, SW_SHOWNOACTIVATE);
        f.state().canvas.borrow_mut().select_tool(Tool::Brush);
        assert!(views(f.state()).is_none());
        f.state().canvas.borrow_mut().select_tool(Tool::Select);
        assert!(views(f.state()).is_some());
        super::super::edit_mode::execute(f.hwnd(), f.state(), false).unwrap();
        assert!(views(f.state()).is_none());
        let _ = ShowWindow(f.document.panel.0, SW_HIDE);
        super::super::edit_mode::toolbar_mode(f.hwnd(), f.state(), false).unwrap();
        {
            let mut c = f.state().canvas.borrow_mut();
            let view = crate::session_minified::View {
                size: crate::crop_geometry::Size {
                    width: 100.,
                    height: 100.,
                },
                offset: (0., 0.),
            };
            c.minified = Some(
                crate::desktop::canvas::minified::Presentation::restored(
                    c.image.clone(),
                    crate::session_minified::Views {
                        image: view,
                        annotations: view,
                    },
                    view.size,
                    &c.editor,
                )
                .unwrap(),
            );
        }
        assert!(views(f.state()).is_none());
        f.state().canvas.borrow_mut().minified = None;
        f.send(WM_LBUTTONDOWN, p);
        assert_eq!(GetCapture(), f.hwnd());
        let old = f.view();
        f.state().extensions.borrow_mut().snapshot = None;
        assert!(views(f.state()).is_none());
        super::super::extension_overlay::sync(f.hwnd(), f.state()).unwrap();
        assert!(GetCapture().is_invalid());
        assert!(!old.dragging());
        assert_eq!(old.selected_text().unwrap(), None);
        assert!(f.state().error.borrow().is_none());
    }
}
#[test]
fn copy_rechecks_source_selection_and_failure_reports_notice_without_accessing_clipboard() {
    unsafe {
        let f = Fixture::new();
        let v = f.view();
        let p = f.point();
        v.begin(p, false).unwrap();
        v.finish([205., p[1]], Modifiers::default()).unwrap();
        let text = v.selected_text().unwrap().unwrap();
        copy_with(f.hwnd(), f.state(), &v, &text, true, |actual, current| {
            assert_eq!(actual, "Native overlay");
            assert!(current());
            v.cancel(true);
            assert!(!current());
            Ok(())
        })
        .unwrap();
        copy_with(
            f.hwnd(),
            f.state(),
            &v,
            "Native overlay",
            false,
            |_, current| {
                assert!(current());
                Err(windows::core::Error::new(
                    E_FAIL,
                    "Injected clipboard failure",
                ))
            },
        )
        .unwrap();
        copy_with(
            f.hwnd(),
            f.state(),
            &v,
            "Native overlay",
            false,
            |_, current| {
                assert!(current());
                f.state().extensions.borrow_mut().snapshot = None;
                assert!(!current());
                Ok(())
            },
        )
        .unwrap();
        assert!(f.state().error.borrow().is_none());
    }
}
#[test]
fn current_text_selection_consumes_ctrl_c_before_image_copy_without_touching_clipboard() {
    unsafe {
        let f = Fixture::new();
        let p = f.point();
        let v = f.view();
        let _ = ShowWindow(f.hwnd(), SW_SHOWNOACTIVATE);
        let edit = crate::desktop::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                windows::core::w!("EDIT"),
                windows::core::w!("previous text"),
                WS_CHILD | WS_VISIBLE,
                0,
                100,
                80,
                20,
                Some(f.hwnd()),
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let _ = SetFocus(Some(edit.0));
        assert_eq!(GetFocus(), edit.0);
        f.send(WM_LBUTTONDOWN, p);
        assert_eq!(
            GetFocus(),
            f.hwnd(),
            "overlay click must reclaim EDIT focus"
        );
        f.send(WM_MOUSEMOVE, [205., p[1]]);
        f.send(WM_LBUTTONUP, [205., p[1]]);
        let mut original = [0; 256];
        GetKeyboardState(&mut original).unwrap();
        struct Restore([u8; 256]);
        impl Drop for Restore {
            fn drop(&mut self) {
                unsafe {
                    SetKeyboardState(&self.0).unwrap();
                }
            }
        }
        let _restore = Restore(original);
        let mut keys = [0; 256];
        keys[VK_CONTROL.0 as usize] = 0x80;
        SetKeyboardState(&keys).unwrap();
        // Repeat/release/WM_CHAR must be consumed, but may not publish the clipboard again.
        for (msg, wp, lp) in [
            (WM_KEYDOWN, 0x43, 1 << 30),
            (WM_KEYUP, 0x43, 0),
            (WM_CHAR, 3, 0),
        ] {
            SendMessageW(f.hwnd(), msg, Some(WPARAM(wp)), Some(LPARAM(lp)));
            assert!(f.state().copy_job.borrow().is_none());
            assert_eq!(
                v.selected_text().unwrap().as_deref(),
                Some("Native overlay")
            );
        }
        SendMessageW(
            f.hwnd(),
            WM_NCLBUTTONDOWN,
            Some(WPARAM(HTNOWHERE as usize)),
            Some(LPARAM(0)),
        );
        assert_eq!(v.selected_text().unwrap(), None);
        SendMessageW(f.hwnd(), WM_KEYDOWN, Some(WPARAM(0x43)), Some(LPARAM(0)));
        assert!(f.state().copy_job.borrow().is_some());
        crate::desktop::copy_job::cancel(f.state());
    }
}
