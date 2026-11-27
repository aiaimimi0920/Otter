//! Select display format and inspect exact encoded size before choosing a destination.
use crate::desktop::{
    export::codec::{Format, Source},
    parameter_slider::{Control, Range},
};
use crate::pixels::Pixels;
use windows::{
    core::{Result, HSTRING},
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
    },
};
#[path = "export_save_template.rs"]
mod layout;
#[path = "export_preview.rs"]
mod worker;
const USER: WINDOW_LONG_PTR_INDEX =
    WINDOW_LONG_PTR_INDEX((2 * std::mem::size_of::<isize>()) as i32);
const TIMER: usize = 1;
struct Panel {
    preview: worker::Preview,
    slider: Option<Control>,
    quality: u8,
    value: Option<(Format, Pixels)>,
    fatal: Option<windows::core::Error>,
}
pub(super) unsafe fn choose(hwnd: HWND, source: Source) -> Result<Option<(Format, Pixels)>> {
    let template = layout::template();
    let mut panel = Panel {
        preview: worker::Preview::new(source),
        slider: None,
        quality: 85,
        value: None,
        fatal: None,
    };
    let result = DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(hwnd),
        Some(procedure),
        LPARAM(&mut panel as *mut Panel as isize),
    );
    if let Some(e) = panel.fatal {
        return Err(e);
    }
    if result == -1 {
        return Err(windows::core::Error::from_win32());
    }
    Ok(panel.value)
}
unsafe fn label(hwnd: HWND, id: i32, s: &str) -> Result<()> {
    SetDlgItemTextW(hwnd, id, &HSTRING::from(s))
}
unsafe fn update(hwnd: HWND, p: *mut Panel) -> Result<()> {
    let active = (*p).preview.poll();
    let fmt = (*p).preview.desired;
    let slider = (*p).slider.as_ref().unwrap().handle();
    let _ = EnableWindow(
        slider,
        matches!(fmt, Format::Jpeg(_) | Format::WebpLossy(_)),
    );
    let msg = match &(*p).preview.result {
        None => crate::ui_text::text("export-size-pending").into(),
        Some(Err(e)) => {
            crate::ui_text::format("export-size-failed", &[("error", e.clone().into())])
        }
        Some(Ok(bytes)) => crate::ui_text::format(
            "export-size-exact",
            &[("bytes", bytes.len().to_string().into())],
        ),
    };
    let ok = (*p).preview.result.as_ref().is_some_and(|r| r.is_ok());
    let _ = EnableWindow(GetDlgItem(Some(hwnd), IDOK.0)?, ok);
    label(hwnd, 201, &msg)?;
    if active {
        if SetTimer(Some(hwnd), TIMER, 80, None) == 0 {
            return Err(windows::core::Error::from_win32());
        }
    } else {
        let _ = KillTimer(Some(hwnd), TIMER);
    }
    Ok(())
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if msg == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, USER, lp.0);
    }
    let p = GetWindowLongPtrW(hwnd, USER) as *mut Panel;
    if p.is_null() {
        return 0;
    }
    if msg == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, USER, 0);
        return 0;
    }
    let id = (wp.0 & 0xffff) as i32;
    if msg == WM_CLOSE || msg == WM_COMMAND && id == IDCANCEL.0 {
        let _ = EndDialog(hwnd, 0);
        return 1;
    }
    let result = (|| -> Result<bool> {
        match msg {
            WM_INITDIALOG => {
                let mut rect = RECT {
                    left: 12,
                    top: 43,
                    right: 285,
                    bottom: 60,
                };
                MapDialogRect(hwnd, &mut rect)?;
                let range = Range::new(85., Some(1.), Some(100.), 1.).unwrap();
                (*p).slider = Some(Control::create(hwnd, rect, range)?);
                windows::Win32::UI::Controls::CheckRadioButton(hwnd, 101, 104, 101)?;
            }
            WM_COMMAND if (101..=104).contains(&id) => {
                (*p).preview.request(match id {
                    101 => Format::Png,
                    102 => Format::Jpeg((*p).quality),
                    103 => Format::WebpLossless,
                    _ => Format::WebpLossy((*p).quality),
                });
            }
            WM_HSCROLL => {
                if !matches!((*p).preview.desired, Format::Jpeg(_) | Format::WebpLossy(_)) {
                    return Ok(false);
                }
                if let Some(q) = (*p)
                    .slider
                    .as_ref()
                    .and_then(|s| s.changed(lp))
                    .and_then(|s| s.parse::<u8>().ok())
                {
                    (*p).quality = q;
                    (*p).preview
                        .request(if matches!((*p).preview.desired, Format::WebpLossy(_)) {
                            Format::WebpLossy(q)
                        } else {
                            Format::Jpeg(q)
                        });
                }
            }
            WM_TIMER if wp.0 == TIMER => {}
            WM_COMMAND if id == IDOK.0 => {
                if let Some(Ok(bytes)) = (*p).preview.result.as_ref() {
                    (*p).value = Some(((*p).preview.desired, bytes.clone()));
                    let _ = EndDialog(hwnd, 1);
                }
                return Ok(true);
            }
            _ => return Ok(false),
        }
        if msg != WM_TIMER {
            label(
                hwnd,
                200,
                &crate::ui_text::format(
                    if matches!((*p).preview.desired, Format::Jpeg(_)) {
                        "export-quality-white"
                    } else if matches!((*p).preview.desired, Format::WebpLossy(_)) {
                        "export-quality-webp"
                    } else {
                        "export-quality-lossless"
                    },
                    &[("quality", (*p).quality.to_string().into())],
                ),
            )?;
        }
        update(hwnd, p)?;
        Ok(true)
    })();
    match result {
        Ok(used) => used as isize,
        Err(e) => {
            (*p).fatal = Some(e);
            let _ = EndDialog(hwnd, 0);
            1
        }
    }
}
