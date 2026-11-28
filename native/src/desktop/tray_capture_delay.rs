//! Tray countdown presets use the shared capture scheduler.
use super::*;
use crate::desktop::capture_countdown as delay;
use windows::core::HSTRING;
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let sub = Menu(CreatePopupMenu()?);
    let active = delay::active();
    for (id, key, enabled) in [
        (
            delay::THREE,
            "tray-delay-three",
            !active && delay::ready(state),
        ),
        (
            delay::FIVE,
            "tray-delay-five",
            !active && delay::ready(state),
        ),
    ] {
        AppendMenuW(
            sub.0,
            MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
            id,
            &HSTRING::from(super::actions::tray_label(
                state,
                id,
                crate::ui_text::text(key),
            )),
        )?;
    }
    AppendMenuW(
        menu,
        MF_POPUP,
        sub.0 .0 as usize,
        &HSTRING::from(crate::ui_text::text("tray-delay")),
    )?;
    std::mem::forget(sub);
    Ok(())
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, id: usize) -> Result<bool> {
    match id {
        delay::THREE | delay::FIVE => delay::start(hwnd, state, id)?,
        _ => return Ok(false),
    }
    Ok(true)
}
