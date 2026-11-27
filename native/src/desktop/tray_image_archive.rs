//! Archive authority is granted locally for this run; imported settings never enable file writes.
use super::*;
use crate::desktop::export_image_autosave as archive;
use windows::core::HSTRING;
const COPY: usize = 0x1ef4;
const PIN: usize = 0x1ef5;
const DIRECTORY: usize = 0x1ef6;
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let sub = Menu(CreatePopupMenu()?);
    let (copy, pin) = {
        let a = state.image_archive.borrow();
        (a.copy, a.pin)
    };
    for (id, key, checked) in [
        (COPY, "archive-copy", copy),
        (PIN, "archive-pin", pin),
        (DIRECTORY, "archive-directory", false),
    ] {
        AppendMenuW(
            sub.0,
            MF_STRING | if checked { MF_CHECKED } else { MF_UNCHECKED },
            id,
            &HSTRING::from(crate::ui_text::text(key)),
        )?;
    }
    AppendMenuW(
        menu,
        MF_POPUP,
        sub.0 .0 as usize,
        &HSTRING::from(crate::ui_text::text("archive-menu")),
    )?;
    std::mem::forget(sub);
    Ok(())
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, id: usize) -> Result<bool> {
    if !matches!(id, COPY | PIN | DIRECTORY) {
        return Ok(false);
    }
    if state.modals.blocked() || state.exit_requested.get() {
        return Ok(true);
    }
    let Some(_guard) = crate::desktop::file_dialog::Guard::begin(&state.file_dialog) else {
        return Ok(true);
    };
    if let Err(e) = configure(hwnd, state, id) {
        if state.lifetime.owns(hwnd) {
            MessageBoxW(
                Some(hwnd),
                &HSTRING::from(crate::ui_text::format(
                    "archive-config-failed",
                    &[("error", e.message().to_string().into())],
                )),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
    Ok(true)
}
unsafe fn configure(hwnd: HWND, state: &State, id: usize) -> Result<()> {
    let err = |e: String| Error::new(E_FAIL, e);
    if id == DIRECTORY {
        if let Some(path) = crate::desktop::session_asset_dialog::choose_path(
            hwnd,
            crate::ui_text::text("archive-directory"),
        )? {
            if state.lifetime.owns(hwnd) && !state.exit_requested.get() {
                state.image_archive.borrow_mut().target = Some(archive::grant(&path).map_err(err)?);
            }
        }
    } else {
        let (on, path) = {
            let a = state.image_archive.borrow();
            (
                if id == COPY { a.copy } else { a.pin },
                a.directory().map_err(err)?,
            )
        };
        if !on {
            let message = crate::ui_text::format(
                "archive-consent",
                &[("directory", path.to_string_lossy().into_owned().into())],
            );
            if MessageBoxW(
                Some(hwnd),
                &HSTRING::from(message),
                w!("Otter"),
                MB_YESNO | MB_DEFBUTTON2 | MB_ICONQUESTION,
            ) != IDYES
                || !state.lifetime.owns(hwnd)
                || state.exit_requested.get()
            {
                return Ok(());
            }
        }
        let mut a = state.image_archive.borrow_mut();
        if !on {
            a.confirm(&path).map_err(err)?;
        }
        if id == COPY {
            a.copy = !on;
        } else {
            a.pin = !on;
        }
    }
    Ok(())
}
