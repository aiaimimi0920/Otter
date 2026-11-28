//! One-stop Neuro-themed image export dialog: select format, quality, folder, and filename in a single view.
use crate::desktop::{
    dialog_controls,
    export::codec::{Format, Source},
    session_asset_dialog,
};
use crate::pixels::Pixels;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};
use windows::{
    core::{Result, HSTRING},
    Win32::{
        Foundation::*,
        Graphics::Gdi::InvalidateRect,
        UI::{
            Controls::DRAWITEMSTRUCT,
            Input::KeyboardAndMouse::{EnableWindow, SetFocus},
            WindowsAndMessaging::*,
        },
    },
};

#[path = "export_save_template.rs"]
mod layout;
#[path = "export_preview.rs"]
mod worker;

const USER: WINDOW_LONG_PTR_INDEX =
    WINDOW_LONG_PTR_INDEX((2 * std::mem::size_of::<isize>()) as i32);
const TIMER: usize = 1;
const PATH_LIMIT: usize = 4096;

thread_local! {
    static LAST_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub(super) struct Choice {
    pub format: Format,
    pub bytes: Pixels,
    pub path: PathBuf,
}

struct Panel {
    preview: worker::Preview,
    slider: Option<layout::QualitySlider>,
    quality: u8,
    default_dir: PathBuf,
    stem: String,
    custom_status: Option<String>,
    settled_size_text: Option<String>,
    pending_since: Option<std::time::Instant>,
    shown_format: Option<Format>,
    shown_slider_enabled: Option<bool>,
    shown_ok_enabled: Option<bool>,
    shown_quality_label: String,
    shown_status_label: String,
    value: Option<Choice>,
    fatal: Option<windows::core::Error>,
}

pub(super) unsafe fn choose(
    hwnd: HWND,
    source: Source,
    stem: String,
    default_dir: PathBuf,
) -> Result<Option<Choice>> {
    let initial_dir = LAST_DIR
        .with_borrow(|d| d.clone())
        .filter(|d| d.is_dir())
        .unwrap_or(default_dir);
    let template = layout::template();
    let mut panel = Panel {
        preview: worker::Preview::new(source),
        slider: None,
        quality: 85,
        default_dir: initial_dir,
        stem,
        custom_status: None,
        settled_size_text: None,
        pending_since: Some(std::time::Instant::now()),
        shown_format: None,
        shown_slider_enabled: None,
        shown_ok_enabled: None,
        shown_quality_label: String::new(),
        shown_status_label: String::new(),
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

fn unique_filename(dir: &Path, stem: &str, ext: &str) -> String {
    let base = format!("{stem}.{ext}");
    if dir.join(&base).symlink_metadata().is_err() {
        return base;
    }
    for n in 2..=999 {
        let candidate = format!("{stem}_{n}.{ext}");
        if dir.join(&candidate).symlink_metadata().is_err() {
            return candidate;
        }
    }
    base
}

fn swap_extension(current_name: &str, new_ext: &str, dir: &Path, fallback_stem: &str) -> String {
    let trimmed = current_name.trim();
    if trimmed.is_empty() {
        return unique_filename(dir, fallback_stem, new_ext);
    }
    let stem = ["png", "jpg", "jpeg", "webp"]
        .into_iter()
        .find_map(|ext| {
            let suffix = format!(".{ext}");
            trimmed
                .get(..trimmed.len().checked_sub(suffix.len())?)
                .filter(|_| trimmed[trimmed.len() - suffix.len()..].eq_ignore_ascii_case(&suffix))
        })
        .unwrap_or(trimmed);
    let candidate = format!("{stem}.{new_ext}");
    if dir.join(&candidate).symlink_metadata().is_err() {
        candidate
    } else {
        unique_filename(dir, stem, new_ext)
    }
}

unsafe fn read_field(hwnd: HWND, id: i32) -> String {
    let mut buf = vec![0u16; PATH_LIMIT + 2];
    let len = GetDlgItemTextW(hwnd, id, &mut buf) as usize;
    String::from_utf16_lossy(&buf[..len])
}

unsafe fn label(hwnd: HWND, id: i32, s: &str) -> Result<()> {
    SetDlgItemTextW(hwnd, id, &HSTRING::from(s))
}

unsafe fn update(hwnd: HWND, p: &mut Panel) -> Result<()> {
    let active = p.preview.poll();
    let fmt = p.preview.desired;
    let slider_enabled = matches!(fmt, Format::Jpeg(_) | Format::WebpLossy(_));
    if p.shown_slider_enabled != Some(slider_enabled) {
        p.shown_slider_enabled = Some(slider_enabled);
        if let Some(slider) = p.slider.as_ref() {
            slider.set_enabled(slider_enabled);
        }
    }
    let prev_id = p.shown_format.map(|old| unsafe { active_format_id(old) });
    let next_id = active_format_id(fmt);
    if prev_id != Some(next_id) {
        p.shown_format = Some(fmt);
        for id in prev_id.into_iter().chain(std::iter::once(next_id)) {
            if let Ok(btn) = GetDlgItem(Some(hwnd), id) {
                let _ = InvalidateRect(Some(btn), None, false);
            }
        }
    }
    if let Some(Ok(bytes)) = &p.preview.result {
        p.settled_size_text = Some(crate::ui_text::format(
            "export-size-exact",
            &[("bytes", bytes.len().to_string().into())],
        ));
        p.pending_since = None;
    }
    let msg = if let Some(err) = p.custom_status.as_ref() {
        err.clone()
    } else {
        match &p.preview.result {
            Some(Err(e)) => {
                crate::ui_text::format("export-size-failed", &[("error", e.clone().into())])
            }
            Some(Ok(_)) => p.settled_size_text.clone().unwrap_or_default(),
            None => {
                let debounce = p
                    .pending_since
                    .is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(250));
                if debounce {
                    if let Some(prev) = p.settled_size_text.as_ref() {
                        prev.clone()
                    } else {
                        crate::ui_text::text("export-size-pending").into()
                    }
                } else {
                    crate::ui_text::text("export-size-pending").into()
                }
            }
        }
    };
    // Keep Export enabled across quick format/quality switches so IDOK never flashes disabled/enabled.
    let ok = !matches!(p.preview.result, Some(Err(_)));
    if p.shown_ok_enabled != Some(ok) {
        p.shown_ok_enabled = Some(ok);
        if let Ok(btn) = GetDlgItem(Some(hwnd), IDOK.0) {
            let _ = EnableWindow(btn, ok);
            let _ = InvalidateRect(Some(btn), None, false);
        }
    }
    if p.shown_status_label != msg {
        p.shown_status_label = msg.clone();
        label(hwnd, 201, &msg)?;
    }
    if active {
        if SetTimer(Some(hwnd), TIMER, 16, None) == 0 {
            return Err(windows::core::Error::from_win32());
        }
    } else {
        let _ = KillTimer(Some(hwnd), TIMER);
    }
    Ok(())
}

fn validate_target(
    dir_text: &str,
    file_text: &str,
    format: Format,
) -> std::result::Result<PathBuf, &'static str> {
    let dir_str = dir_text.trim();
    let file_str = file_text.trim();
    if dir_str.is_empty()
        || file_str.is_empty()
        || dir_str.len() > PATH_LIMIT
        || file_str.len() > 240
    {
        return Err(crate::ui_text::text("export-path-invalid"));
    }
    if file_str.contains(['/', '\\', ':', '\0']) || file_str == "." || file_str == ".." {
        return Err(crate::ui_text::text("export-path-invalid"));
    }
    let dir = PathBuf::from(dir_str);
    if !dir.is_absolute() {
        return Err(crate::ui_text::text("export-path-invalid"));
    }
    if !dir.exists() && std::fs::create_dir_all(&dir).is_err() {
        return Err(crate::ui_text::text("export-path-invalid"));
    }
    if !dir.is_dir() {
        return Err(crate::ui_text::text("export-path-invalid"));
    }
    let has_dot = Path::new(file_str).extension().is_some();
    let final_name = if has_dot {
        file_str.to_owned()
    } else {
        format!("{file_str}.{}", format.ext())
    };
    let full = dir.join(final_name);
    if !format.accepts(&full) {
        return Err(crate::ui_text::text("export-extension-mismatch"));
    }
    if full.symlink_metadata().is_ok() {
        return Err(crate::ui_text::text("app-workspace-destination-exists"));
    }
    Ok(full)
}

unsafe fn active_format_id(fmt: Format) -> i32 {
    match fmt {
        Format::Png => 101,
        Format::Jpeg(_) => 102,
        Format::WebpLossless => 103,
        Format::WebpLossy(_) => 104,
    }
}

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if msg == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, USER, lp.0);
    }
    if let Some(bg) = dialog_controls::background(msg, wp, lp) {
        return bg;
    }
    let p = GetWindowLongPtrW(hwnd, USER) as *mut Panel;
    if p.is_null() {
        return 0;
    }
    if msg == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, USER, 0);
        return 0;
    }
    let panel = &mut *p;
    if msg == WM_DRAWITEM {
        let item = &*(lp.0 as *const DRAWITEMSTRUCT);
        let id = item.CtlID as i32;
        let active = id == IDOK.0 || id == active_format_id(panel.preview.desired);
        dialog_controls::draw_button(item, active);
        return 1;
    }
    let id = (wp.0 & 0xffff) as i32;
    let notify = ((wp.0 >> 16) & 0xffff) as u16;
    if msg == WM_CLOSE || (msg == WM_COMMAND && id == IDCANCEL.0) {
        let _ = EndDialog(hwnd, 0);
        return 1;
    }
    let result = (|| -> Result<bool> {
        match msg {
            WM_INITDIALOG => {
                let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
                for btn_id in [101, 102, 103, 104, layout::BROWSE_ID, IDOK.0, IDCANCEL.0] {
                    if let Ok(btn) = GetDlgItem(Some(hwnd), btn_id) {
                        dialog_controls::button(btn)?;
                    }
                }
                for edit_id in [layout::DIR_EDIT_ID, layout::FILE_EDIT_ID] {
                    if let Ok(edit) = GetDlgItem(Some(hwnd), edit_id) {
                        dialog_controls::edit(edit)?;
                        SendMessageW(edit, 0x00C5, Some(WPARAM(PATH_LIMIT)), Some(LPARAM(0)));
                    }
                }
                let mut rect = RECT {
                    left: 12,
                    top: 62,
                    right: 312,
                    bottom: 78,
                };
                MapDialogRect(hwnd, &mut rect)?;
                panel.slider = Some(layout::QualitySlider::create(hwnd, rect, panel.quality)?);
                let dir_str = panel.default_dir.to_string_lossy().into_owned();
                label(hwnd, layout::DIR_EDIT_ID, &dir_str)?;
                let initial_file =
                    unique_filename(&panel.default_dir, &panel.stem, panel.preview.desired.ext());
                label(hwnd, layout::FILE_EDIT_ID, &initial_file)?;
                if let Ok(edit) = GetDlgItem(Some(hwnd), layout::FILE_EDIT_ID) {
                    let _ = SetFocus(Some(edit));
                }
            }
            WM_COMMAND if (101..=104).contains(&id) => {
                panel.custom_status = None;
                let next = match id {
                    101 => Format::Png,
                    102 => Format::Jpeg(panel.quality),
                    103 => Format::WebpLossless,
                    _ => Format::WebpLossy(panel.quality),
                };
                if panel.preview.desired != next {
                    panel.pending_since = Some(std::time::Instant::now());
                    panel.preview.request(next);
                }
                let dir = PathBuf::from(read_field(hwnd, layout::DIR_EDIT_ID).trim());
                let cur_file = read_field(hwnd, layout::FILE_EDIT_ID);
                let updated = swap_extension(&cur_file, next.ext(), &dir, &panel.stem);
                if cur_file != updated {
                    label(hwnd, layout::FILE_EDIT_ID, &updated)?;
                }
            }
            WM_COMMAND if id == layout::QUALITY_ID && notify == layout::QUALITY_CHANGED => {
                if let Some(slider) = panel.slider.as_ref() {
                    let q = slider.value();
                    panel.quality = q;
                    panel.custom_status = None;
                    if matches!(
                        panel.preview.desired,
                        Format::Jpeg(_) | Format::WebpLossy(_)
                    ) {
                        panel.pending_since = Some(std::time::Instant::now());
                        panel.preview.request(
                            if matches!(panel.preview.desired, Format::WebpLossy(_)) {
                                Format::WebpLossy(q)
                            } else {
                                Format::Jpeg(q)
                            },
                        );
                    }
                }
            }
            WM_COMMAND if id == layout::BROWSE_ID => {
                if let Ok(Some(picked)) = session_asset_dialog::choose_path(
                    hwnd,
                    crate::ui_text::text("export-folder-label"),
                ) {
                    panel.custom_status = None;
                    let picked_str = picked.to_string_lossy().into_owned();
                    label(hwnd, layout::DIR_EDIT_ID, &picked_str)?;
                    let cur_file = read_field(hwnd, layout::FILE_EDIT_ID);
                    let updated = swap_extension(
                        &cur_file,
                        panel.preview.desired.ext(),
                        &picked,
                        &panel.stem,
                    );
                    label(hwnd, layout::FILE_EDIT_ID, &updated)?;
                    LAST_DIR.with_borrow_mut(|slot| *slot = Some(picked));
                }
            }
            WM_COMMAND
                if matches!(id, layout::DIR_EDIT_ID | layout::FILE_EDIT_ID)
                    && notify == EN_CHANGE as u16 =>
            {
                if panel.custom_status.take().is_some() {
                    update(hwnd, panel)?;
                }
                return Ok(true);
            }
            WM_TIMER if wp.0 == TIMER => {}
            WM_COMMAND if id == IDOK.0 => {
                if panel.preview.result.is_none() {
                    panel.preview.settle();
                }
                let Some(Ok(bytes)) = panel.preview.result.as_ref() else {
                    update(hwnd, panel)?;
                    return Ok(true);
                };
                let bytes = bytes.clone();
                let dir_text = read_field(hwnd, layout::DIR_EDIT_ID);
                let file_text = read_field(hwnd, layout::FILE_EDIT_ID);
                match validate_target(&dir_text, &file_text, panel.preview.desired) {
                    Ok(path) => {
                        if let Some(parent) = path.parent() {
                            LAST_DIR.with_borrow_mut(|slot| *slot = Some(parent.to_path_buf()));
                        }
                        panel.value = Some(Choice {
                            format: panel.preview.desired,
                            bytes,
                            path,
                        });
                        let _ = EndDialog(hwnd, 1);
                    }
                    Err(msg) => {
                        panel.custom_status = Some(msg.to_owned());
                        update(hwnd, panel)?;
                    }
                }
                return Ok(true);
            }
            _ => return Ok(false),
        }
        if msg != WM_TIMER {
            let text: String = match panel.preview.desired {
                Format::Jpeg(_) => crate::ui_text::format(
                    "export-quality-white",
                    &[("quality", panel.quality.to_string().into())],
                ),
                Format::WebpLossy(_) => crate::ui_text::format(
                    "export-quality-webp",
                    &[("quality", panel.quality.to_string().into())],
                ),
                Format::Png | Format::WebpLossless => {
                    crate::ui_text::text("export-quality-lossless").into()
                }
            };
            if panel.shown_quality_label != text {
                panel.shown_quality_label = text.clone();
                label(hwnd, 200, &text)?;
            }
        }
        update(hwnd, panel)?;
        Ok(true)
    })();
    match result {
        Ok(used) => used as isize,
        Err(e) => {
            panel.fatal = Some(e);
            let _ = EndDialog(hwnd, 0);
            1
        }
    }
}

#[cfg(test)]
#[path = "export_save_dialog_tests.rs"]
mod tests;
