//! 原生字体选择；默认配置与选中文字事务分开，模态期间不借用画布。
use super::{canvas::Canvas, text_input::Pending, State};
use crate::{editor::Tool, text_annotation::TextFont};
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{Controls::Dialogs::*, WindowsAndMessaging::*},
    },
};
pub(super) const COMMAND: usize = 0x101F;
pub(super) unsafe fn append_menu(menu: HMENU, canvas: &Canvas) -> Result<()> {
    let selected = Pending::selected(canvas).is_some();
    AppendMenuW(
        menu,
        MF_STRING
            | if selected || matches!(canvas.tool, Tool::Text | Tool::Serial) {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        COMMAND,
        if selected {
            w!("修改选中文字字体…")
        } else if canvas.tool == Tool::Serial {
            w!("新序号默认字体…")
        } else {
            w!("新文字默认字体…")
        },
    )
}
pub(super) unsafe fn show(hwnd: HWND, state: &State) {
    if state.canvas.borrow().minified.is_some() {
        return;
    }
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    let (pending, initial, serial_default) = {
        let mut canvas = state.canvas.borrow_mut();
        canvas.cancel_gesture();
        let pending = Pending::selected(&canvas);
        if pending.is_none() && !matches!(canvas.tool, Tool::Text | Tool::Serial) {
            return;
        }
        let serial_default = pending.is_none() && canvas.tool == Tool::Serial;
        let initial = pending.as_ref().and_then(|value| value.text()).map_or_else(
            || {
                if serial_default {
                    TextFont::new(canvas.tool_profiles.serial_font_family(), 16.0, false).unwrap()
                } else {
                    canvas.tool_profiles.text_font.clone()
                }
            },
            |text| text.font().clone(),
        );
        (pending, initial, serial_default)
    };
    let result = (|| -> Result<()> {
        let Some(font) = choose(hwnd, &initial, serial_default)? else {
            return Ok(());
        };
        if !IsWindow(Some(hwnd)).as_bool() {
            return Ok(());
        }
        if let Some(pending) = pending {
            let text = pending.text().unwrap().with_font(font);
            let size = super::annotation_text::measure(&text)?;
            pending.commit(&mut state.canvas.borrow_mut(), text, size)?;
        } else {
            let mut canvas = state.canvas.borrow_mut();
            if serial_default
                && canvas.tool == Tool::Serial
                && canvas.tool_profiles.serial_font_family() == initial.family()
            {
                canvas.tool_profiles.set_serial_font_family(font.family());
            } else if !serial_default
                && canvas.tool == Tool::Text
                && canvas.tool_profiles.text_font == initial
            {
                canvas.tool_profiles.text_font = font;
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        if IsWindow(Some(hwnd)).as_bool() {
            MessageBoxW(
                Some(hwnd),
                &HSTRING::from(format!("无法应用字体。\n{error}")),
                w!("Otter"),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    if IsWindow(Some(hwnd)).as_bool() {
        if let Err(error) = super::hit_region::update(hwnd, state) {
            eprintln!("native-text-font-region: {error}");
        }
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}
unsafe fn choose(hwnd: HWND, font: &TextFont, family_only: bool) -> Result<Option<TextFont>> {
    let mut log = LOGFONTW {
        lfHeight: -(font.font_size()
            * windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(96) as f32
            / 96.0)
            .round() as i32,
        lfWeight: if font.bold() { 700 } else { 500 },
        ..Default::default()
    };
    let name: Vec<u16> = font.family().encode_utf16().collect();
    if name.len() >= log.lfFaceName.len() {
        return Err(Error::new(
            E_INVALIDARG,
            "Font family exceeds Windows font dialog limit",
        ));
    }
    log.lfFaceName[..name.len()].copy_from_slice(&name);
    let mut options = CHOOSEFONTW {
        lStructSize: std::mem::size_of::<CHOOSEFONTW>() as u32,
        hwndOwner: hwnd,
        lpLogFont: &mut log,
        iPointSize: (font.font_size() * 7.5).round() as i32,
        Flags: CF_SCREENFONTS
            | CF_INITTOLOGFONTSTRUCT
            | CF_LIMITSIZE
            | CF_FORCEFONTEXIST
            | CF_NOSCRIPTSEL
            | if family_only {
                CF_NOSIZESEL | CF_NOSTYLESEL
            } else {
                CHOOSEFONT_FLAGS(0)
            },
        nSizeMin: 1,
        nSizeMax: 749,
        ..Default::default()
    };
    if !ChooseFontW(&mut options).as_bool() {
        let error = CommDlgExtendedError();
        return if error == COMMON_DLG_ERRORS(0) {
            Ok(None)
        } else {
            Err(Error::new(E_FAIL, format!("Font dialog failed: {error:?}")))
        };
    }
    if family_only {
        // 禁用的属性不从对话框结果回写；序号字号仍由半径派生。
        log.lfItalic = 0;
        log.lfUnderline = 0;
        log.lfStrikeOut = 0;
        log.lfWeight = 500;
        options.iPointSize = 120;
    }
    decode(&log, options.iPointSize).map(Some)
}
fn decode(log: &LOGFONTW, point_size: i32) -> Result<TextFont> {
    if log.lfItalic != 0
        || log.lfUnderline != 0
        || log.lfStrikeOut != 0
        || !matches!(log.lfWeight, 0 | 400 | 500 | 700)
    {
        return Err(Error::new(
            E_INVALIDARG,
            "Only regular and bold text styles are currently supported",
        ));
    }
    let end = log
        .lfFaceName
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(log.lfFaceName.len());
    let name = String::from_utf16(&log.lfFaceName[..end])
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid font family encoding"))?;
    TextFont::new(&name, point_size as f32 / 7.5, log.lfWeight >= 700)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid font selection"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn font_dialog_points_convert_to_document_pixels_and_unsupported_styles_are_rejected() {
        let mut log = LOGFONTW {
            lfWeight: 700,
            ..Default::default()
        };
        for (target, unit) in log.lfFaceName.iter_mut().zip("微软雅黑".encode_utf16()) {
            *target = unit;
        }
        let font = decode(&log, 120).unwrap();
        assert_eq!(font.font_size(), 16.0);
        assert!(font.bold());
        assert_eq!(font.family(), "微软雅黑");
        assert!(decode(&log, 0).is_err());
        log.lfItalic = 1;
        assert!(decode(&log, 120).is_err());
    }
}
