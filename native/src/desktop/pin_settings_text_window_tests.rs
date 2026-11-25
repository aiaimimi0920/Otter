//! Real BUTTON/commit credentials/feedback reclamation; inject a receiver only at the clipboard boundary without reading/writing the user's clipboard.
use super::projection::{caption, tooltip, LanguageReset};
use super::*;
use crate::desktop::pin_settings::text as cached;
use crate::ui_text::{self, DisplayLanguage};
use crate::unit_attachment_store::tests as store_fixture;
use std::cell::Cell;
use windows::Win32::{Graphics::Gdi::*, UI::Controls::*};
struct Dib {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
}
impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            if !self.old.is_invalid() {
                SelectObject(self.dc, self.old);
            }
            if !self.bitmap.is_invalid() {
                assert!(DeleteObject(self.bitmap.into()).as_bool());
            }
            assert!(DeleteDC(self.dc).as_bool());
        }
    }
}
unsafe fn painted(hwnd: HWND) -> Vec<u8> {
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect).unwrap();
    assert!((1..=512).contains(&rect.right) && (1..=512).contains(&rect.bottom));
    let mut dib = Dib {
        dc: CreateCompatibleDC(None),
        bitmap: HBITMAP::default(),
        old: HGDIOBJ::default(),
    };
    assert!(!dib.dc.is_invalid());
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: rect.right,
            biHeight: -rect.bottom,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    dib.bitmap = CreateDIBSection(Some(dib.dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
    dib.old = SelectObject(dib.dc, dib.bitmap.into());
    let length = rect.right as usize * rect.bottom as usize * 4;
    std::ptr::write_bytes(bits.cast::<u8>(), 0xa5, length);
    let draw = DRAWITEMSTRUCT {
        CtlType: ODT_BUTTON,
        itemAction: ODA_DRAWENTIRE,
        hwndItem: hwnd,
        hDC: dib.dc,
        rcItem: rect,
        ..Default::default()
    };
    assert_eq!(
        SendMessageW(
            GetParent(hwnd).unwrap(),
            WM_DRAWITEM,
            None,
            Some(LPARAM(&draw as *const _ as isize))
        )
        .0,
        1
    );
    assert!(GdiFlush().as_bool());
    let pixels = std::slice::from_raw_parts(bits.cast::<u8>(), length).to_vec();
    assert!(pixels.iter().any(|v| *v != 0xa5));
    pixels
}
unsafe fn populated() -> Fixture {
    let f = Fixture::new();
    let raw = serde_json::value::RawValue::from_string("{\"revision\":4}".into()).unwrap();
    *f.state.extension_document.borrow_mut() = crate::desktop::extension_document::Owner::restore(
        store_fixture::store(crate::sticker_extra_text::tests::attachments(
            " 原文😀\n全文 ",
            " 译文\n全文 ",
        )),
        Some(&raw),
    );
    f
}
unsafe fn invoke(
    f: &Fixture,
    window: HWND,
    id: u16,
    publish: impl FnOnce(&str, &dyn Fn() -> bool) -> Result<()>,
) {
    SendMessageW(field(window, id), BM_CLICK, None, None);
    let mut message = MSG::default();
    assert!(PeekMessageW(
        &mut message,
        Some(f.owner.0),
        super::super::super::super::EXECUTE,
        super::super::super::super::EXECUTE,
        PM_REMOVE
    )
    .as_bool());
    assert_eq!(message.wParam.0, id as usize);
    assert_eq!(message.lParam.0, neuro_mini_ui::PANEL);
    settings::execute_with(f.owner.0, &f.state, id, |owner, state, id| {
        cached::copy_with(owner, state, id, publish)
    })
    .unwrap();
    refresh(&f.state);
}
#[test]
fn actual_copy_buttons_share_a_row_copy_hidden_full_text_and_keep_document_and_tool_state() {
    let _reset = LanguageReset(ui_text::current_language());
    for language in DisplayLanguage::ALL {
        ui_text::set_language(language);
        unsafe {
            let f = populated();
            let window = open(&f);
            let stamp = f.state.canvas.borrow().editor.document_stamp();
            let attachment = f.state.extension_document.borrow().stamp();
            let mut first = RECT::default();
            let mut second = RECT::default();
            let controls = [
                field(window, cached::OCR),
                field(window, cached::TRANSLATION),
            ];
            for display in DisplayLanguage::ALL {
                ui_text::set_language(display);
                refresh(&f.state);
                for ((id, key), hwnd) in [
                    (cached::OCR, "pin-copy-ocr"),
                    (cached::TRANSLATION, "pin-copy-translation"),
                ]
                .into_iter()
                .zip(controls)
                {
                    assert_eq!(field(window, id), hwnd);
                    assert_eq!(caption(hwnd), ui_text::text(key));
                    assert_eq!(tooltip(window, hwnd), ui_text::text(key));
                }
                GetWindowRect(field(window, cached::OCR), &mut first).unwrap();
                GetWindowRect(field(window, cached::TRANSLATION), &mut second).unwrap();
                assert_eq!(first.top, second.top);
                assert!(first.right < second.left);
            }
            ui_text::set_language(language);
            refresh(&f.state);
            for (id, expected) in [
                (cached::OCR, " 原文😀\n全文 "),
                (cached::TRANSLATION, " 译文\n全文 "),
            ] {
                assert!(IsWindowEnabled(field(window, id)).as_bool());
                let before = painted(field(window, id));
                invoke(&f, window, id, |text, current| {
                    assert!(current());
                    assert_eq!(text, expected);
                    Ok(())
                });
                // The Action's HWND name is an accessibility label; visible feedback requires checking actual owner-drawn output.
                assert_ne!(painted(field(window, id)), before);
                assert_eq!(caption(field(window, id)), ui_text::text("pin-text-copied"));
                assert_eq!(
                    tooltip(window, field(window, id)),
                    ui_text::text("pin-text-copied")
                );
            }
            assert!(stamp.matches(&f.state.canvas.borrow().editor));
            assert!(attachment.matches(&f.state.extension_document.borrow()));
            assert_eq!(f.state.canvas.borrow().tool, crate::editor::Tool::Select);
            assert!(!crate::desktop::extension_commands::busy(&f.state));
            assert!(f.state.error.borrow().is_none());
        }
    }
}
#[test]
fn compact_copy_feedback_keeps_complete_accessible_names_and_fits_native_font() {
    let _reset = LanguageReset(ui_text::current_language());
    unsafe {
        let f = populated();
        let panel = open(&f);
        let control = field(panel, cached::OCR);
        for language in DisplayLanguage::ALL {
            ui_text::set_language(language);
            for copied in [true, false] {
                invoke(&f, panel, cached::OCR, |_, _| {
                    if copied {
                        Ok(())
                    } else {
                        Err(windows::core::Error::new(
                            E_ACCESSDENIED,
                            "synthetic copy failure",
                        ))
                    }
                });
                assert_eq!(field(panel, cached::OCR), control);
                let label = ui_text::text(if copied {
                    "pin-text-copied"
                } else {
                    "pin-text-copy-failed"
                });
                assert_eq!(caption(control), label);
                assert_eq!(tooltip(panel, control), label);
                let value: Vec<_> = ui_text::text(if copied {
                    "pin-text-copied"
                } else {
                    "pin-text-copy-failed-short"
                })
                .encode_utf16()
                .collect();
                let dc = GetDC(Some(control));
                assert!(!dc.is_invalid());
                // Width 58 is not mini-ui's <48 compact variant; icon-free feedback
                // uses this normal font and the entire unpadded client text frame.
                let font = HGDIOBJ(SendMessageW(control, WM_GETFONT, None, None).0 as *mut _);
                let old = SelectObject(dc, font);
                let mut size = SIZE::default();
                let measured = GetTextExtentPoint32W(dc, &value, &mut size).as_bool();
                SelectObject(dc, old);
                assert_eq!(ReleaseDC(Some(control), dc), 1);
                let mut rect = RECT::default();
                GetClientRect(control, &mut rect).unwrap();
                assert!(measured);
                assert!(
                    size.cx <= rect.right,
                    "{language:?}: {size:?} exceeds {rect:?}"
                );
            }
        }
    }
}
#[test]
fn empty_or_stale_translation_disables_only_its_button_and_queued_old_copy_is_rejected() {
    unsafe {
        let f = Fixture::new();
        let window = open(&f);
        for id in [cached::OCR, cached::TRANSLATION] {
            assert!(!IsWindowEnabled(field(window, id)).as_bool());
        }
        assert!(IsWindowEnabled(field(window, settings::OPACITY)).as_bool());
        let f = populated();
        let window = open(&f);
        let called = Cell::new(false);
        let before = painted(field(window, cached::TRANSLATION));
        invoke(&f, window, cached::TRANSLATION, |_, current| {
            // A source generation change before publication must invalidate the same production credential without clearing the original clipboard.
            f.state
                .extension_document
                .borrow_mut()
                .store
                .visibility(&[("neuro.official/ocr.result", true)])
                .unwrap();
            assert!(!current());
            called.set(true);
            Ok(())
        });
        assert!(called.get());
        assert_eq!(painted(field(window, cached::TRANSLATION)), before);
        SendMessageW(field(window, cached::OCR), BM_CLICK, None, None);
        f.state.extension_document.borrow_mut().store = Default::default();
        settings::execute_with(f.owner.0, &f.state, cached::OCR, |_, _, _| {
            panic!("stale click reached clipboard")
        })
        .unwrap();
        refresh(&f.state);
        assert!(!IsWindowEnabled(field(window, cached::OCR)).as_bool());
    }
}
#[test]
fn failed_copy_has_local_feedback_timer_resets_it_and_toolbar_close_cleans_it_up() {
    unsafe {
        let f = populated();
        let window = open(&f);
        let before = painted(field(window, cached::OCR));
        invoke(&f, window, cached::OCR, |_, _| {
            Err(windows::core::Error::new(
                E_ACCESSDENIED,
                "synthetic clipboard busy",
            ))
        });
        let failed = painted(field(window, cached::OCR));
        assert_ne!(failed, before);
        SendMessageW(f.owner.0, WM_TIMER, Some(WPARAM(cached::TIMER)), None);
        assert_eq!(painted(field(window, cached::OCR)), failed);
        std::thread::sleep(std::time::Duration::from_millis(1650));
        SendMessageW(f.owner.0, WM_TIMER, Some(WPARAM(cached::TIMER)), None);
        assert_eq!(painted(field(window, cached::OCR)), before);
        invoke(&f, window, cached::OCR, |_, current| {
            assert!(current());
            Ok(())
        });
        let success = painted(field(window, cached::OCR));
        assert_ne!(success, before);
        assert_ne!(success, failed);
        close(f.owner.0, &f.state);
        assert!(!settings::is_open(&f.state));
        assert!(KillTimer(Some(f.owner.0), cached::TIMER).is_err());
        assert_eq!(f.state.canvas.borrow().tool, crate::editor::Tool::Pan);
    }
}
