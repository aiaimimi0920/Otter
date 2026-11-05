//! 不可变提交租约与真实分层样式重入；所有窗口隐藏，不读取用户 profile。
use super::*;
use crate::desktop::{software_surface::SoftwareSurface, Apartment, Window};
use std::cell::{Cell, RefCell};
use windows::{
    core::w,
    Win32::{
        Graphics::Direct2D::{Common::*, *},
        System::LibraryLoader::GetModuleHandleW,
    },
};

fn surface(size: u32) -> SoftwareSurface {
    let factory: ID2D1Factory =
        unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap() };
    SoftwareSurface::new(
        &factory,
        D2D_SIZE_U {
            width: size,
            height: size,
        },
    )
    .unwrap()
}
fn red(target: &ID2D1RenderTarget) -> Result<()> {
    unsafe {
        target.Clear(Some(&D2D1_COLOR_F {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }));
    }
    Ok(())
}

#[test]
fn pending_pixels_cannot_be_overwritten_and_cancel_releases_lease() {
    let _apartment = Apartment::new().unwrap();
    let mut surface = surface(16);
    surface.draw(red).unwrap();
    assert!(surface.prepare_present(false, 0.5).is_err());
    let frame = surface.prepare_present(false, 1.0).unwrap();
    assert!(surface.prepare_present(false, 1.0).is_err());
    assert!(surface
        .draw(|_| panic!("pending DIB must not be drawn"))
        .is_err());
    assert!(frame.bitmap.pixels().iter().all(|p| *p == [0, 0, 255, 255]));
    drop(frame);
    surface.draw(red).unwrap();
    assert!(surface
        .prepare_present(false, 1.0)
        .unwrap()
        .present(HWND::default())
        .is_err());
    surface.draw(red).unwrap(); // 提交失败同样释放租约，但必须重新绘制才能再次提交。
}

#[test]
fn resize_and_renderer_drop_do_not_invalidate_pending_dib() {
    let _apartment = Apartment::new().unwrap();
    let mut surface = surface(16);
    surface.draw(red).unwrap();
    let first = surface.prepare_present(true, 1.0).unwrap();
    let weak = Rc::downgrade(&first.bitmap);
    surface
        .resize(D2D_SIZE_U {
            width: 32,
            height: 32,
        })
        .unwrap();
    surface
        .draw(|target| {
            unsafe {
                target.Clear(Some(&D2D1_COLOR_F {
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                }));
            }
            Ok(())
        })
        .unwrap();
    let second = surface.prepare_present(true, 1.0).unwrap();
    drop(surface);
    assert_eq!(first.bitmap.size.width, 16);
    assert!(first.bitmap.pixels().iter().all(|p| *p == [0, 0, 255, 255]));
    assert!(second
        .bitmap
        .pixels()
        .iter()
        .all(|p| *p == [0, 255, 0, 255]));
    assert!(weak.upgrade().is_some());
    drop(first);
    assert!(
        weak.upgrade().is_none(),
        "DIB lifetime must end at its last owner"
    );
}

thread_local! {
    static OWNER: RefCell<Option<SoftwareSurface>> = const { RefCell::new(None) };
    static STYLE_CALLS: Cell<u32> = const { Cell::new(0) };
    static BORROW_FAILED: Cell<bool> = const { Cell::new(false) };
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_STYLECHANGING {
        STYLE_CALLS.set(STYLE_CALLS.get().saturating_add(1));
        OWNER.with(|owner| match owner.try_borrow_mut() {
            Ok(mut surface) => {
                surface.take();
            } // 模拟重入中丢弃渲染资源，不碰提交中的 DIB。
            Err(_) => BORROW_FAILED.set(true),
        });
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        OWNER.with(|owner| {
            owner.borrow_mut().take();
        });
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Software.Submission.Test"), Some(self.0));
        }
    }
}
#[test]
fn layered_style_reenters_after_model_borrow_is_released() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Software.Submission.Test"),
                lpfnWndProc: Some(procedure),
                ..Default::default()
            }),
            0
        );
        let _class = Class(instance);
        let window = Window(
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP | WS_EX_NOACTIVATE,
                w!("Otter.Software.Submission.Test"),
                w!(""),
                WS_POPUP,
                0,
                0,
                16,
                16,
                None,
                None,
                Some(instance),
                None,
            )
            .unwrap(),
        );
        OWNER.with(|owner| *owner.borrow_mut() = Some(surface(32)));
        let frame = OWNER.with(|owner| {
            let mut owner = owner.borrow_mut();
            let surface = owner.as_mut().unwrap();
            surface.draw(red).unwrap();
            surface.prepare_present(true, 1.0).unwrap()
        });
        STYLE_CALLS.set(0);
        BORROW_FAILED.set(false);
        // 实测 ULW resize 不保证产生 WM_SIZE；用生产必经的样式通知验证真实同步重入。
        let transition =
            crate::desktop::software_surface::window_style::Transition::begin(window.0, true)
                .unwrap();
        frame.present(window.0).unwrap();
        transition.commit().unwrap();
        assert!(
            STYLE_CALLS.get() > 0,
            "must exercise real synchronous WM_STYLECHANGING"
        );
        assert!(!BORROW_FAILED.get());
        OWNER.with(|owner| assert!(owner.borrow().is_none()));
        let mut rect = RECT::default();
        GetWindowRect(window.0, &mut rect).unwrap();
        assert_eq!((rect.right - rect.left, rect.bottom - rect.top), (32, 32));
    }
}
