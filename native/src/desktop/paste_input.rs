//! Explicit image paste: copy only bounded bytes under the clipboard lock, then confirm and hand off to the existing decoder worker outside it.
use super::State;
#[path = "clipboard_file.rs"]
pub(super) mod file;
use crate::pixels::Pixels;
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{
        Foundation::*,
        System::{DataExchange::*, Memory::*},
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
pub(super) const COMMAND: usize = 0x1041;
pub(super) const REPLACE: usize = 0x1050;
struct Clipboard;
impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}
struct Lock(HGLOBAL);
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalUnlock(self.0);
        }
    }
}

pub(super) enum Snapshot {
    Editable(std::sync::Arc<super::clipboard_editable::Content>),
    Png(Pixels),
    Dib(Pixels),
    Files(Vec<std::path::PathBuf>),
}
pub(super) unsafe fn read_image(hwnd: HWND, state: &State, new_pin: bool) -> Result<Snapshot> {
    let format = RegisterClipboardFormatW(w!("PNG"));
    if format == 0 {
        return Err(Error::from_win32());
    }
    OpenClipboard(Some(hwnd))?;
    let _clipboard = Clipboard;
    // Replacing an image retains the existing flattened-image contract.
    if new_pin {
        if let Some(content) = state
            .editable_clipboard
            .borrow_mut()
            .matching(GetClipboardSequenceNumber())
        {
            return Ok(Snapshot::Editable(content));
        }
    }
    let (format, dib) = if IsClipboardFormatAvailable(format).is_ok() {
        (format, false)
    } else if IsClipboardFormatAvailable(17).is_ok() {
        (17, true)
    } else if IsClipboardFormatAvailable(8).is_ok() {
        (8, true)
    } else if IsClipboardFormatAvailable(15).is_ok() {
        (15, false)
    } else {
        return Err(Error::new(
            E_INVALIDARG,
            crate::ui_text::text("action-paste-no-image"),
        ));
    };
    let handle = GetClipboardData(format)?;
    let memory = HGLOBAL(handle.0);
    let size = GlobalSize(memory);
    let limit = if format == 15 {
        file::MAX_BYTES
    } else {
        257 * 1024 * 1024
    };
    if !(8..=limit).contains(&size) {
        return Err(Error::new(
            E_INVALIDARG,
            "Clipboard image exceeds input budget",
        ));
    }
    let mut lease = crate::pixels::reserve(size)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Clipboard read budget exhausted"))?;
    let pointer = GlobalLock(memory);
    if pointer.is_null() {
        return Err(Error::from_win32());
    }
    let _lock = Lock(memory); // The system still owns the handle; GlobalFree is forbidden.
    let source = std::slice::from_raw_parts(pointer.cast::<u8>(), size);
    if format == 15 {
        return file::parse(source).map(Snapshot::Files);
    }
    if !dib && !source.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::new(E_INVALIDARG, "Invalid clipboard PNG signature"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate clipboard input"))?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::new(
            E_OUTOFMEMORY,
            "Clipboard input capacity exceeds budget",
        ));
    }
    bytes.extend_from_slice(source);
    let bytes = Pixels::from_reserved(bytes, lease);
    Ok(if dib {
        Snapshot::Dib(bytes)
    } else {
        Snapshot::Png(bytes)
    })
}
pub(super) unsafe fn start(hwnd: HWND, state: &State) {
    start_with(hwnd, state, true);
}
pub(super) unsafe fn start_with(hwnd: HWND, state: &State, new_pin: bool) {
    if !new_pin && !super::open_image::can_replace(state) {
        return;
    }
    if state.open_job.borrow().is_some() || (new_pin && state.new_pin.borrow().is_some()) {
        return;
    }
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    let result = (|| -> Result<()> {
        // Sample before reading the clipboard/preparing images; all formats use the same physical coordinates, not fixed (100,100).
        let mut cursor = POINT::default();
        if new_pin {
            GetCursorPos(&mut cursor)?;
        }
        let snapshot = read_image(hwnd, state, new_pin)?;
        if !new_pin && !super::open_image::confirm_replace(hwnd, state) {
            return Ok(());
        }
        state.canvas.borrow_mut().cancel_gesture();
        if GetCapture() == hwnd {
            ReleaseCapture()?;
        }
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
        deliver_at(hwnd, state, snapshot, new_pin, cursor)
    })();
    if let Err(error) = result {
        if IsWindow(Some(hwnd)).as_bool() {
            MessageBoxW(
                Some(hwnd),
                &HSTRING::from(crate::ui_text::format(
                    "action-paste-failed",
                    &[("error", error.to_string().into())],
                )),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
}
// Smoke/regression tests inject only snapshots and coordinates, without accessing the user's clipboard or moving the mouse; production uses deliver_at.
pub(super) unsafe fn deliver(
    hwnd: HWND,
    state: &State,
    snapshot: Snapshot,
    new_pin: bool,
) -> Result<()> {
    deliver_at(hwnd, state, snapshot, new_pin, POINT::default())
}
pub(super) unsafe fn deliver_at(
    hwnd: HWND,
    state: &State,
    snapshot: Snapshot,
    new_pin: bool,
    cursor: POINT,
) -> Result<()> {
    use super::open_job::{start_positioned, Input};
    let placement = if let Snapshot::Editable(content) = &snapshot {
        state
            .editable_clipboard
            .borrow()
            .placement(content, (cursor.x, cursor.y))
    } else {
        state
            .editable_clipboard
            .borrow()
            .position((cursor.x, cursor.y))
    };
    let input = match snapshot {
        Snapshot::Editable(content) if new_pin => Input::Editable(content, None),
        Snapshot::Editable(_) => {
            return Err(Error::new(
                E_INVALIDARG,
                "Editable paste requires a new pin",
            ))
        }
        Snapshot::Png(bytes) => Input::Encoded(bytes),
        Snapshot::Dib(bytes) => Input::Dib(bytes),
        Snapshot::Files(mut paths) => {
            if new_pin {
                Input::Files(paths)
            } else if paths.len() == 1 {
                Input::File(paths.pop().unwrap())
            } else {
                return Err(Error::new(
                    E_INVALIDARG,
                    crate::ui_text::text("action-paste-one-file"),
                ));
            }
        }
    };
    start_positioned(
        hwnd,
        state,
        input,
        new_pin,
        new_pin.then_some((cursor, placement)),
    )
}
