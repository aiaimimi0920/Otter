//! Actual hidden HWNDs verify commit/rollback; no GPU fault injection or changes to external windows/foreground focus.
use super::*;
use windows::core::w;

#[test]
fn no_redirection_window_only_allows_layered_submission_and_preserves_its_flag() {
    unsafe {
        let window = Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let original = style(window.0).unwrap();
        {
            let error = match Transition::begin(window.0, false) {
                Ok(_) => panic!("must reject a GPU-only HWND before mutation"),
                Err(error) => error,
            };
            assert_eq!(error.code(), E_INVALIDARG);
            assert!(error.message().contains("replacement"));
            assert_eq!(style(window.0).unwrap(), original);
        }
        {
            let _pending = Transition::begin(window.0, true).unwrap();
            assert_eq!(style(window.0).unwrap(), original | WS_EX_LAYERED.0);
        }
        assert_eq!(style(window.0).unwrap(), original);
        Transition::begin(window.0, true).unwrap().commit().unwrap();
        assert_eq!(style(window.0).unwrap(), original | WS_EX_LAYERED.0);
    }
}

struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.0).unwrap() };
    }
}

#[test]
fn gpu_transition_rolls_back_only_before_association_and_honors_owner_revocation() {
    unsafe {
        let window = Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP | WS_EX_LAYERED,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let original = style(window.0).unwrap();
        let active = std::cell::Cell::new(true);
        let owns = || active.get();
        {
            let _pending = Transition::gpu_guarded(window.0, &owns).unwrap();
            assert_eq!(style(window.0).unwrap(), original & !WS_EX_LAYERED.0);
        }
        assert_eq!(style(window.0).unwrap(), original);
        Transition::gpu_guarded(window.0, &owns)
            .unwrap()
            .commit()
            .unwrap();
        // A subsequent candidate failure must not make an already committed transaction restore layered.
        assert_eq!(style(window.0).unwrap(), original & !WS_EX_LAYERED.0);
        Transition::begin(window.0, true).unwrap().commit().unwrap();
        let pending = Transition::gpu_guarded(window.0, &owns).unwrap();
        active.set(false);
        assert!(pending.commit().is_err());
        assert_eq!(style(window.0).unwrap(), original & !WS_EX_LAYERED.0);
        assert!(Transition::gpu_guarded(window.0, &owns).is_err());
        assert!(!IsWindowVisible(window.0).as_bool());
    }
}

#[test]
fn transaction_preserves_unowned_bits_geometry_and_visibility() {
    unsafe {
        for pinned in [false, true] {
            let window = Window(
                CreateWindowExW(
                    WS_EX_TOOLWINDOW,
                    w!("STATIC"),
                    w!(""),
                    WS_POPUP,
                    17,
                    23,
                    64,
                    64,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            let original = style(window.0).unwrap();
            let mut before = RECT::default();
            GetWindowRect(window.0, &mut before).unwrap();
            {
                let _pending = Transition::begin(window.0, pinned).unwrap();
                assert_eq!(
                    style(window.0).unwrap() & MASK,
                    if pinned { WS_EX_LAYERED.0 } else { 0 }
                );
                // Rollback must not erase styles set by other owners during the transaction.
                SetWindowLongPtrW(
                    window.0,
                    GWL_EXSTYLE,
                    (style(window.0).unwrap() | WS_EX_NOACTIVATE.0) as isize,
                );
            }
            assert_eq!(style(window.0).unwrap(), original | WS_EX_NOACTIVATE.0);
            Transition::begin(window.0, pinned)
                .unwrap()
                .commit()
                .unwrap();
            assert_eq!(
                style(window.0).unwrap() & MASK,
                if pinned { WS_EX_LAYERED.0 } else { 0 }
            );
            let mut after = RECT::default();
            GetWindowRect(window.0, &mut after).unwrap();
            assert_eq!(before, after);
            assert!(!IsWindowVisible(window.0).as_bool());
            // Reentering the same software mode leaves committed state unchanged.
            Transition::begin(window.0, pinned)
                .unwrap()
                .commit()
                .unwrap();
        }
        assert!(Transition::begin(HWND::default(), true).is_err());
    }
}

#[test]
fn unwind_rolls_back_and_wrong_thread_is_rejected() {
    unsafe {
        let window = Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let original = style(window.0).unwrap();
        let pending = Transition::begin(window.0, true).unwrap();
        // This isolated fixture uses an unguarded transaction; verify only unwind style rollback without reusing captured mutable state.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _pending = pending;
            panic!("fixture");
        }));
        assert!(result.is_err());
        assert_eq!(style(window.0).unwrap(), original);
        let raw = window.0 .0 as usize;
        assert!(std::thread::spawn(move || {
            Transition::begin(HWND(raw as *mut _), true).is_err()
        })
        .join()
        .unwrap());
        assert_eq!(style(window.0).unwrap(), original);
    }
}

#[test]
fn software_frame_commits_after_style_transition_and_failed_commit_rolls_back() {
    use crate::desktop::{software_surface::SoftwareSurface, Apartment};
    use windows::Win32::Graphics::Direct2D::{Common::*, *};
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        for pinned in [false, true] {
            let window = Window(
                CreateWindowExW(
                    WS_EX_TOOLWINDOW,
                    w!("STATIC"),
                    w!(""),
                    WS_POPUP,
                    0,
                    0,
                    32,
                    32,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            let original = style(window.0).unwrap();
            let pending = Transition::begin(window.0, pinned).unwrap();
            // Simulate a reentrant owner changing this transaction's bits; commit must reject and roll back rather than silently accept.
            SetWindowLongPtrW(
                window.0,
                GWL_EXSTYLE,
                (style(window.0).unwrap() ^ WS_EX_LAYERED.0) as isize,
            );
            assert!(pending.commit().is_err());
            assert_eq!(style(window.0).unwrap(), original);
            let mut surface = SoftwareSurface::new(
                &factory,
                D2D_SIZE_U {
                    width: 32,
                    height: 32,
                },
            )
            .unwrap();
            let pending = Transition::begin(window.0, pinned).unwrap();
            surface
                .draw(|target| {
                    target.Clear(Some(&D2D1_COLOR_F {
                        r: 0.25,
                        g: 0.5,
                        b: 0.75,
                        a: 1.0,
                    }));
                    Ok(())
                })
                .unwrap();
            surface.present(window.0, pinned, 1.0).unwrap();
            pending.commit().unwrap();
            assert!(!IsWindowVisible(window.0).as_bool());
        }
    }
}
