//! Built-in language choices enqueue one local owner request; menu callbacks never write settings or rebuild UI.
use super::*;
use crate::display_language::DisplayLanguage;
use windows::core::HSTRING;

const ENGLISH: usize = 0x10e0;
const SIMPLIFIED_CHINESE: usize = 0x10e1;
const LANGUAGES: [(usize, DisplayLanguage); 2] = [
    (ENGLISH, DisplayLanguage::English),
    (SIMPLIFIED_CHINESE, DisplayLanguage::SimplifiedChinese),
];

pub(super) unsafe fn append(menu: HMENU) -> Result<()> {
    let submenu = Menu(CreatePopupMenu()?);
    for (command, language) in LANGUAGES {
        AppendMenuW(
            submenu.0,
            MF_STRING,
            command,
            &HSTRING::from(language.name()),
        )?;
    }
    let selected = LANGUAGES
        .iter()
        .find(|(_, language)| *language == crate::ui_text::current_language())
        .expect("all built-in display languages have menu entries")
        .0;
    CheckMenuRadioItem(
        submenu.0,
        ENGLISH as u32,
        SIMPLIFIED_CHINESE as u32,
        selected as u32,
        MF_BYCOMMAND.0,
    )?;
    AppendMenuW(
        menu,
        MF_POPUP,
        submenu.0 .0 as usize,
        &HSTRING::from(crate::ui_text::text("tray-language")),
    )?;
    // The parent HMENU now owns the submenu, including failure cleanup in its RAII wrapper.
    std::mem::forget(submenu);
    Ok(())
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<bool> {
    let Some((_, language)) = LANGUAGES.iter().find(|(id, _)| *id == command) else {
        return Ok(false);
    };
    if !IsWindow(Some(hwnd)).as_bool() || state.modals.blocked() {
        return Ok(true);
    }
    let previous = state.display_language_request.replace(Some(*language));
    if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
        state.display_language_request.set(previous);
        return Err(error);
    }
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_submenu_uses_autonyms_and_radio_state() {
        unsafe {
            let menu = Menu(CreatePopupMenu().unwrap());
            append(menu.0).unwrap();
            let tree = crate::native_menu::snapshot(menu.0).unwrap();
            let children = tree[0].children.as_ref().unwrap();
            assert_eq!(children.len(), 2);
            for (entry, (command, language)) in children.iter().zip(LANGUAGES) {
                let selected = language == crate::ui_text::current_language();
                assert_eq!(entry.command, command);
                assert_eq!(entry.label, language.name());
                // CheckMenuRadioItem clears both flags on every unselected entry.
                assert_eq!(entry.radio, selected);
                assert_eq!(entry.checked, selected);
            }
        }
    }
}
