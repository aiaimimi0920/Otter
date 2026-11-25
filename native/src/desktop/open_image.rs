//! Native opening defaults to new pins, confirming only explicit replacement; hold no RefCell across modals.
use super::{hit_region::HitRegion, image::Image, State};
use std::path::Path;
#[path = "open_paths.rs"]
mod paths;
use windows::{
    core::{w, Result, HSTRING, PCWSTR, PWSTR},
    Win32::{
        Foundation::*,
        Graphics::Gdi::InvalidateRect,
        UI::{Controls::Dialogs::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

pub(super) struct PreparedDocument {
    pub(super) record_history: bool,
    pub(super) image: Image,
    pub(super) region: HitRegion,
    pub(super) capture_origin: Option<POINT>,
    pub(super) paste_origin: Option<POINT>,
    pub(super) capture_source: Option<super::canvas::CaptureSource>,
    pub(super) editable: Option<super::clipboard_editable::Prepared>,
    pub(super) placement: Option<super::clipboard_placement::Request>,
}

#[cfg(test)]
#[path = "open_image_tests.rs"]
mod tests;
impl PreparedDocument {
    #[cfg(test)]
    pub(super) fn from_capture(
        captured: crate::capture_job::Captured,
        pinned: bool,
        origin: Option<POINT>,
    ) -> Result<Self> {
        Self::from_capture_with_cancel(captured, pinned, origin, None, || false)
    }
    pub(super) fn from_capture_with_cancel(
        captured: crate::capture_job::Captured,
        pinned: bool,
        origin: Option<POINT>,
        context: Option<crate::capture_job::Context>,
        cancelled: impl Fn() -> bool,
    ) -> Result<Self> {
        let context = context.unwrap_or_else(|| {
            crate::capture_job::Context::region(
                origin,
                captured.preview.width,
                captured.preview.height,
                captured.backend,
            )
        });
        let metadata = context.metadata(captured.original.is_some(), captured.downgraded);
        let image = Image {
            width: captured.preview.width,
            height: captured.preview.height,
            pixels: captured.preview.pixels,
        };
        let mut prepared = Self::from_image_with_cancel(image, pinned, cancelled)?;
        prepared.record_history = true;
        prepared.capture_origin = origin;
        prepared.capture_source = Some(super::canvas::CaptureSource {
            session_meta: Some(metadata),
            original_lease: captured.original_lease,
            original: captured.original,
            reason: captured.reason,
            downgraded: captured.downgraded,
            display: captured.display,
            diagnostic: captured.diagnostic,
        });
        Ok(prepared)
    }
    pub(super) fn open(path: &Path, pinned: bool) -> Result<Self> {
        let image = Image::open(path)?;
        Self::from_image(image, pinned)
    }
    pub(super) fn from_image(image: Image, pinned: bool) -> Result<Self> {
        Self::from_image_with_cancel(image, pinned, || false)
    }
    /// Hit masks check the same task-cancellation signal row by row; window closure need not await a whole long-image scan.
    pub(super) fn from_image_with_cancel(
        image: Image,
        pinned: bool,
        cancelled: impl Fn() -> bool,
    ) -> Result<Self> {
        if cancelled() {
            return Err(windows::core::Error::new(
                E_ABORT,
                "Image preparation cancelled",
            ));
        }
        let region = if pinned {
            HitRegion::prepare_cancellable(&image, None, (image.width, image.height), &cancelled)
        } else {
            Ok(HitRegion::default())
        };
        if cancelled() {
            return Err(windows::core::Error::new(
                E_ABORT,
                "Image preparation cancelled",
            ));
        }
        let region = region?;
        Ok(Self {
            record_history: false,
            image,
            region,
            capture_origin: None,
            paste_origin: None,
            capture_source: None,
            editable: None,
            placement: None,
        })
    }
}

pub(super) unsafe fn start(hwnd: HWND, state: &State) {
    start_destination(hwnd, state, true);
}
pub(super) const REPLACE: usize = 0x1051;
pub(super) fn can_replace(state: &State) -> bool {
    !state.group_locked.get() && state.canvas.borrow().minified.is_none()
}
pub(super) unsafe fn start_destination(hwnd: HWND, state: &State, new_pin: bool) {
    if !new_pin && !can_replace(state) {
        return;
    }
    if state.open_job.borrow().is_some() || (new_pin && state.new_pin.borrow().is_some()) {
        return;
    }
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    if let Err(error) = open_with(hwnd, state, new_pin, || choose(hwnd, new_pin)) {
        if IsWindow(Some(hwnd)).as_bool() {
            let text = HSTRING::from(crate::ui_text::format(
                "open-image-failed",
                &[("error", error.to_string().into())],
            ));
            MessageBoxW(
                Some(hwnd),
                PCWSTR(text.as_ptr()),
                w!("Otter"),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}

pub(super) unsafe fn confirm_replace(hwnd: HWND, state: &State) -> bool {
    if !can_replace(state) || state.open_job.borrow().is_some() || !IsWindow(Some(hwnd)).as_bool() {
        return false;
    }
    let has_edits = state.canvas.borrow().editor.has_edit_state();
    let stamp = state.canvas.borrow().editor.document_stamp();
    if has_edits
        && MessageBoxW(
            Some(hwnd),
            &HSTRING::from(crate::ui_text::text("open-image-confirm")),
            w!("Otter"),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
        ) != IDYES
    {
        return false;
    }
    IsWindow(Some(hwnd)).as_bool()
        && can_replace(state)
        && state.open_job.borrow().is_none()
        && stamp.matches(&state.canvas.borrow().editor)
}

pub(super) unsafe fn open_with(
    hwnd: HWND,
    state: &State,
    new_pin: bool,
    choose: impl FnOnce() -> Result<Option<Vec<std::path::PathBuf>>>,
) -> Result<()> {
    if !new_pin && !confirm_replace(hwnd, state) {
        return Ok(());
    }
    state.canvas.borrow_mut().cancel_gesture();
    if GetCapture() == hwnd {
        ReleaseCapture()?;
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
    let stamp = state.canvas.borrow().editor.document_stamp();
    let Some(mut paths) = choose()? else {
        return Ok(());
    };
    if !IsWindow(Some(hwnd)).as_bool() {
        return Ok(());
    }
    if !stamp.matches(&state.canvas.borrow().editor) {
        return Err(windows::core::Error::new(
            E_FAIL,
            crate::ui_text::text("open-image-changed"),
        ));
    }
    let input = if new_pin {
        super::open_job::Input::Files(paths)
    } else if paths.len() == 1 {
        super::open_job::Input::File(paths.pop().unwrap())
    } else {
        return Err(windows::core::Error::new(
            E_INVALIDARG,
            crate::ui_text::text("open-image-single"),
        ));
    };
    super::open_job::start_destination(hwnd, state, input, new_pin)
}

unsafe fn choose(hwnd: HWND, multiple: bool) -> Result<Option<Vec<std::path::PathBuf>>> {
    let mut name = vec![0u16; if multiple { paths::MAX_BUFFER } else { 32768 }];
    let filter: Vec<u16> = format!(
        "{}\0*.png;*.jpg;*.jpeg;*.bmp;*.gif;*.tif;*.tiff;*.webp\0{}\0*.*\0\0",
        crate::ui_text::text("file-filter-images"),
        crate::ui_text::text("file-filter-all")
    )
    .encode_utf16()
    .collect();
    let mut options = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFile: PWSTR(name.as_mut_ptr()),
        nMaxFile: name.len() as u32,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        Flags: OFN_EXPLORER
            | OFN_FILEMUSTEXIST
            | OFN_PATHMUSTEXIST
            | OFN_NOCHANGEDIR
            | if multiple {
                OFN_ALLOWMULTISELECT
            } else {
                OPEN_FILENAME_FLAGS(0)
            },
        ..Default::default()
    };
    if !GetOpenFileNameW(&mut options).as_bool() {
        let error = CommDlgExtendedError();
        if error != COMMON_DLG_ERRORS(0) {
            return Err(windows::core::Error::new(
                E_FAIL,
                format!("Open dialog failed: {error:?}"),
            ));
        }
        return Ok(None);
    }
    paths::parse(&name, options.nFileOffset as usize, multiple).map(Some)
}
