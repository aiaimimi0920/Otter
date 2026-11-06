//! 双击形态切换在几何变更时已有完整代理且宿主被遮罩，返回前恢复原后端。
use super::*;
use std::cell::Cell;
use windows::Win32::{Graphics::Gdi::*, UI::Shell::*};

#[test]
fn failed_preview_allocation_rolls_back_viewport_geometry_and_presentation() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        for restore in [false, true] {
            if restore {
                assert!(minify::collapse(fixture.window.0, &fixture.state, (140, 140)).unwrap());
            }
            let rect = fixture.rect();
            let viewport = fixture.state.canvas.borrow().viewport;
            let opacity = fixture.state.canvas.borrow().window_opacity;
            let mut leases = Vec::new();
            for size in [1024 * 1024, 1024, 1] {
                while let Some(lease) = crate::pixels::reserve(size) {
                    leases.push(lease);
                }
            }
            let result = if restore {
                restore_minified(fixture.window.0, &fixture.state)
            } else {
                minify::collapse(fixture.window.0, &fixture.state, (140, 140))
            };
            drop(leases);
            assert!(result.is_err());
            assert_eq!(fixture.rect(), rect);
            assert_eq!(fixture.state.canvas.borrow().viewport, viewport);
            assert_eq!(fixture.state.canvas.borrow().window_opacity, opacity);
            assert_eq!(fixture.state.canvas.borrow().minified.is_some(), restore);
            assert!(!preview::active(&fixture.state));
        }
    }
}

struct Probe {
    changes: Cell<u32>,
    uncovered: Cell<bool>,
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    if msg == WM_WINDOWPOSCHANGED && lp.0 != 0 {
        let pos = &*(lp.0 as *const WINDOWPOS);
        if !pos.flags.contains(SWP_NOSIZE) {
            let probe = &*(data as *const Probe);
            let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State);
            let region = CreateRectRgn(0, 0, 0, 0);
            let masked = GetWindowRgn(hwnd, region) == NULLREGION;
            let _ = DeleteObject(region.into());
            probe.changes.set(probe.changes.get() + 1);
            probe
                .uncovered
                .set(probe.uncovered.get() || !masked || !preview::active(state));
        }
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}

#[test]
fn double_click_collapse_and_restore_have_complete_frames_on_cpu_and_gpu() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for renderer in [None, Some(crate::desktop::render_commands::SOFTWARE)] {
            let fixture = Fixture::with_renderer(renderer);
            let original = fixture.rect();
            let choice = fixture.state.canvas.borrow().render_choice();
            let stamp = fixture.state.canvas.borrow().editor.document_stamp();
            let probe = Box::new(Probe {
                changes: Cell::new(0),
                uncovered: Cell::new(false),
            });
            SetWindowSubclass(
                fixture.window.0,
                Some(observe),
                556,
                &*probe as *const _ as usize,
            )
            .unwrap();
            for _ in 0..3 {
                for minified in [true, false] {
                    SendMessageW(
                        fixture.window.0,
                        WM_NCLBUTTONDBLCLK,
                        None,
                        Some(LPARAM(140 | (140 << 16))),
                    );
                    assert_eq!(fixture.state.canvas.borrow().minified.is_some(), minified);
                    assert!(!preview::active(&fixture.state));
                    assert!(
                        fixture.state.error.borrow().is_none(),
                        "{:?}",
                        fixture.state.error.borrow()
                    );
                    assert_eq!(fixture.state.canvas.borrow().render_choice(), choice);
                    assert!(stamp.matches(&fixture.state.canvas.borrow().editor));
                    if !minified {
                        assert_eq!(fixture.rect(), original);
                    }
                }
            }
            RemoveWindowSubclass(fixture.window.0, Some(observe), 556).unwrap();
            assert!(probe.changes.get() >= 6);
            assert!(
                !probe.uncovered.get(),
                "host geometry exposed without a complete preview"
            );
        }
    }
}

#[test]
fn minified_drag_keeps_composite_border_and_half_speed_wheel_accumulates() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        let original = fixture.rect();
        fixture.wheel(120);
        assert_eq!(
            fixture.rect().width,
            (f64::from(original.width) * 1.1).round() as u32
        );
        fixture.wheel(-120);
        assert_eq!(fixture.rect(), original);
        assert!(minify::collapse(fixture.window.0, &fixture.state, (140, 140)).unwrap());
        SendMessageW(fixture.window.0, WM_ENTERSIZEMOVE, None, None);
        let proxy = preview::handle(&fixture.state).unwrap();
        let mut rect = RECT::default();
        GetWindowRect(proxy, &mut rect).unwrap();
        let padding = crate::desktop::pin_border_style::width(GetDpiForWindow(fixture.window.0));
        assert_eq!(
            (rect.right - rect.left) as u32,
            fixture.rect().width + 2 * padding
        );
        SendMessageW(fixture.window.0, WM_EXITSIZEMOVE, None, None);
        assert!(!IsWindow(Some(proxy)).as_bool());
    }
}
