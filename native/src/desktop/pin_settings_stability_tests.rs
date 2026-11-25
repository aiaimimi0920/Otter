//! Idle refreshes must neither repeatedly toggle unavailable fields nor relayout or clear user drafts.
use super::*;
use windows::Win32::UI::Controls::{EM_GETSEL, EM_SETSEL};
use windows::Win32::UI::Shell::*;
struct LanguageReset(crate::ui_text::DisplayLanguage);
impl Drop for LanguageReset {
    fn drop(&mut self) {
        crate::ui_text::set_language(self.0);
    }
}
#[test]
fn translated_connections_preserve_numeric_drafts_and_already_queued_edits() {
    use crate::ui_text::{self, DisplayLanguage};
    let _reset = LanguageReset(ui_text::current_language());
    ui_text::set_language(DisplayLanguage::English);
    unsafe {
        let f = Fixture::new();
        let panel = open(&f);
        let width = field(panel, settings::WIDTH);
        let before = history::frame(f.owner.0).unwrap();
        let dpi = GetDpiForWindow(f.owner.0).max(96);
        SetWindowTextW(width, windows::core::w!("456")).unwrap();
        SendMessageW(width, EM_SETSEL, Some(WPARAM(1)), Some(LPARAM(3)));
        let selection = SendMessageW(width, EM_GETSEL, None, None);
        ui_text::set_language(DisplayLanguage::SimplifiedChinese);
        tool_panel::refresh(f.owner.0);
        assert_eq!(field(panel, settings::WIDTH), width);
        let mut text = [0u16; 32];
        let length = GetWindowTextW(width, &mut text) as usize;
        assert_eq!(String::from_utf16_lossy(&text[..length]), "456");
        assert_eq!(SendMessageW(width, EM_GETSEL, None, None), selection);
        // Enqueue under Chinese, refresh to English, then consume the original edit.
        enter(&f, width, "456");
        ui_text::set_language(DisplayLanguage::English);
        tool_panel::refresh(f.owner.0);
        dispatch(&f);
        assert_eq!(
            history::frame(f.owner.0).unwrap(),
            crate::desktop::pin_settings_geometry::width(before, 456, dpi).unwrap()
        );
        assert!(f.state.error.borrow().is_none());
    }
}
#[derive(Default, Debug)]
struct Counts {
    enabled: usize,
    layout: usize,
    paint: usize,
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    p: usize,
) -> LRESULT {
    let counts = &mut *(p as *mut Counts);
    match msg {
        WM_ENABLE => counts.enabled += 1,
        WM_WINDOWPOSCHANGING => counts.layout += 1,
        WM_PAINT => counts.paint += 1,
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
#[test]
fn unavailable_copy_buttons_do_not_toggle_or_repaint_on_unchanged_refresh() {
    unsafe {
        let f = Fixture::new();
        let panel = open(&f);
        let copy = field(panel, settings::text::OCR);
        assert!(!IsWindowEnabled(copy).as_bool());
        let width = field(panel, settings::WIDTH);
        SetWindowTextW(width, windows::core::w!("456")).unwrap();
        let mut counts = Counts::default();
        for h in [panel, copy, width] {
            SetWindowSubclass(h, Some(observe), 607, &mut counts as *mut _ as usize)
                .ok()
                .unwrap();
        }
        for _ in 0..8 {
            tool_panel::refresh(f.owner.0);
        }
        for h in [panel, copy, width] {
            RemoveWindowSubclass(h, Some(observe), 607).ok().unwrap();
        }
        let mut text = [0u16; 16];
        let n = GetWindowTextW(width, &mut text) as usize;
        assert_eq!(String::from_utf16_lossy(&text[..n]), "456");
        assert_eq!(
            counts.enabled, 0,
            "disabled button changed state: {counts:?}"
        );
        assert_eq!(counts.layout, 0, "stable panel moved: {counts:?}");
        assert_eq!(counts.paint, 0, "stable panel repainted: {counts:?}");
    }
}

#[test]
fn refreshed_identical_directory_retires_old_context_without_repainting_controls() {
    unsafe {
        let f = Fixture::new();
        f.state.authorization.borrow_mut().origin = Some("https://loom.example".into());
        let directory = || {
            std::sync::Arc::new(
                crate::projection_delivery::targets::Directory::parse(
                    &serde_json::to_vec(&crate::projection_delivery::targets::tests::body())
                        .unwrap(),
                    "https://loom.example",
                )
                .unwrap(),
            )
        };
        f.state.authorization.borrow_mut().targets.directory = Some(directory());
        let panel = open(&f);
        let controls = [
            panel,
            field(panel, settings::WIDTH),
            field(panel, 205),
            field(panel, 222),
        ];
        for h in controls {
            windows::Win32::Graphics::Gdi::UpdateWindow(h).ok().unwrap();
        }
        let mut counts = Counts::default();
        for h in controls {
            SetWindowSubclass(h, Some(observe), 671, &mut counts as *mut _ as usize)
                .ok()
                .unwrap();
        }
        for n in 0..8 {
            {
                let mut view = f.state.authorization.borrow_mut();
                view.targets.directory = Some(directory());
                view.sender.busy = n % 2 == 0;
                view.editing.busy = n % 2 != 0;
            }
            tool_panel::refresh(f.owner.0);
            for h in controls {
                windows::Win32::Graphics::Gdi::UpdateWindow(h).ok().unwrap();
            }
        }
        for h in controls {
            RemoveWindowSubclass(h, Some(observe), 671).ok().unwrap();
        }
        assert_eq!(counts.enabled, 0, "{counts:?}");
        assert_eq!(counts.layout, 0, "{counts:?}");
        assert_eq!(counts.paint, 0, "{counts:?}");
    }
}
