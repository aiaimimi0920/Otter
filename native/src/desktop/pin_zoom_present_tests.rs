//! 在真实尺寸通知处观察新帧准备顺序；分别覆盖默认 GPU、兼容 GPU 和原子软件提交。
use super::*;
use std::cell::Cell;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};

struct Probe {
    hwnd: HWND,
    expected: Cell<PinRect>,
    previous: Cell<PinRect>,
    changed: Cell<u32>,
    stale: Cell<bool>,
    gpu_prepared: Cell<bool>,
    paints: Cell<u32>,
    stale_clip: Cell<bool>,
}
impl Probe {
    unsafe fn new(fixture: &Fixture) -> Box<Self> {
        let probe = Box::new(Self {
            hwnd: fixture.window.0,
            expected: Cell::new(fixture.rect()),
            previous: Cell::new(fixture.rect()),
            changed: Cell::new(0),
            stale: Cell::new(false),
            gpu_prepared: Cell::new(false),
            paints: Cell::new(0),
            stale_clip: Cell::new(false),
        });
        SetWindowSubclass(probe.hwnd, Some(observe), 549, &*probe as *const _ as usize).unwrap();
        probe
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        unsafe { RemoveWindowSubclass(self.hwnd, Some(observe), 549).unwrap() };
    }
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let probe = &*(data as *const Probe);
    let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State);
    if msg == WM_WINDOWPOSCHANGED && lp.0 != 0 {
        let pos = &*(lp.0 as *const WINDOWPOS);
        if !pos.flags.contains(SWP_NOSIZE) {
            let region = windows::Win32::Graphics::Gdi::CreateRectRgn(0, 0, 0, 0);
            let kind = windows::Win32::Graphics::Gdi::GetWindowRgn(hwnd, region);
            probe
                .stale_clip
                .set(probe.stale_clip.get() || kind != windows::Win32::Graphics::Gdi::RGN_ERROR);
            let _ = windows::Win32::Graphics::Gdi::DeleteObject(region.into());
            let expected = probe.expected.get();
            let factor = GetDpiForWindow(hwnd).max(1) as f32 / 96.0;
            let size = state.canvas.borrow().client_size;
            let ready = size.is_some_and(|size| {
                (size.x * factor - expected.width as f32).abs() < 0.001
                    && (size.y * factor - expected.height as f32).abs() < 0.001
            });
            probe.stale.set(probe.stale.get() || !ready);
            probe.changed.set(probe.changed.get() + 1);
        }
    }
    let result = DefSubclassProc(hwnd, msg, wp, lp);
    if msg == WM_NCCALCSIZE && wp.0 != 0 && !state.canvas.borrow().software_selected() {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let old = probe.previous.get();
            probe.gpu_prepared.set(
                probe.gpu_prepared.get()
                    || (state.canvas.borrow().paints > probe.paints.get()
                        && rect.left == old.x
                        && rect.top == old.y
                        && rect.right - rect.left == old.width as i32
                        && rect.bottom - rect.top == old.height as i32),
            );
        }
    }
    result
}

unsafe fn check(command: Option<usize>, software: bool) {
    let fixture = Fixture::with_renderer(command);
    assert_eq!(fixture.state.canvas.borrow().software_selected(), software);
    if command.is_none() {
        assert!(matches!(
            fixture.state.canvas.borrow().render_choice(),
            crate::desktop::canvas::RenderChoice::Automatic
        ));
    }
    let probe = Probe::new(&fixture);
    let mut expected_zoom = crate::pin_geometry::WheelZoom::default();
    for delta in [120, -120, -120, 120, 30, -30, 240, -240] {
        let before = fixture.rect();
        let expected = expected_zoom
            .next(before, (140, 140), delta as f32 / 120.0)
            .unwrap();
        probe.expected.set(expected);
        probe.previous.set(before);
        probe.gpu_prepared.set(false);
        probe.paints.set(fixture.state.canvas.borrow().paints);
        let changes = probe.changed.get();
        assert!(presentation::apply(
            fixture.window.0,
            &fixture.state,
            expected
        ));
        assert_eq!(fixture.rect(), expected);
        assert!(
            fixture.state.error.borrow().is_none(),
            "{:?}",
            fixture.state.error.borrow()
        );
        assert!(
            !probe.stale.get(),
            "new window geometry exposed before matching frame was drawn"
        );
        assert_eq!(
            probe.changed.get(),
            changes + u32::from(!software),
            "GPU has one geometry change; software must not move the old bitmap first"
        );
        if !software {
            assert!(
                probe.gpu_prepared.get(),
                "GPU frame must be ready while HWND still has old bounds"
            );
        }
        assert!(fixture.state.pin_resize.rect.get().is_none());
        assert!(
            !probe.stale_clip.get(),
            "new HWND geometry must never inherit the old clipping region"
        );
        assert!(!fixture.state.render_active.get());
        assert!(!GetUpdateRect(fixture.window.0, None, false).as_bool());
    }
}

#[test]
fn default_gpu_prepares_zoom_frame_before_hwnd_geometry_changes() {
    let _apartment = Apartment::new().unwrap();
    unsafe { check(None, false) };
}

#[test]
fn compatibility_gpu_prepares_zoom_frame_before_hwnd_geometry_changes() {
    let _apartment = Apartment::new().unwrap();
    unsafe { check(Some(crate::desktop::render_commands::COMPATIBILITY), false) };
}

#[test]
fn software_zoom_submits_new_pixels_and_position_without_moving_old_bitmap() {
    let _apartment = Apartment::new().unwrap();
    unsafe { check(Some(crate::desktop::render_commands::SOFTWARE), true) };
}

#[test]
fn fine_wheel_position_only_changes_keep_the_fractional_geometry() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::with_renderer(None);
        let original = fixture.rect();
        let pointer = (135, 147);
        let mut reference = crate::pin_geometry::WheelZoom::default();
        let mut position_only = 0;
        for delta in [1i16; 240].into_iter().chain([-1; 240]) {
            let before = fixture.rect();
            let expected = reference
                .next(before, pointer, delta as f32 / 120.0)
                .unwrap();
            if before != expected
                && before.width == expected.width
                && before.height == expected.height
            {
                position_only += 1;
            }
            fixture.wheel_at(delta, pointer);
            assert_eq!(
                fixture.rect(),
                expected,
                "position-only commits must not reset fractional zoom"
            );
            assert!(fixture.state.error.borrow().is_none());
        }
        assert!(
            position_only > 0,
            "fixture must exercise move-only rounding steps"
        );
        assert_eq!(fixture.rect(), original);
    }
}
