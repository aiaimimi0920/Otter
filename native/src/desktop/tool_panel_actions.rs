//! Tool-panel property/document-operation catalog; command IDs share menus, and labels read actual styles only.
use super::{context_menu as menu, State};
use windows::{
    core::{Result, HSTRING},
    Win32::{Foundation::HWND, UI::WindowsAndMessaging::*},
};
pub(super) const ACTIONS: &[(usize, &str)] = &[
    (menu::COLOR, "common-color"),
    (menu::FILL_COLOR, "tool-action-fill-color"),
    (menu::WIDTH_DOWN, "tool-action-width-down"),
    (menu::WIDTH_UP, "tool-action-width-up"),
    (0x4F, "common-opacity"),
    (0x46, "tool-action-fill"),
    (menu::APPLY_STYLE, "tool-action-apply-style"),
    (menu::OPEN, "tool-action-open-image"),
    (super::copy_job::COMMAND, "tool-action-copy-image"),
    (menu::EXPORT, "tool-action-export-png"),
    (super::numeric_style::WIDTH, "tool-action-exact-width"),
    (super::numeric_style::OPACITY, "tool-action-exact-opacity"),
    (super::capture_input::DEFAULT, "tool-action-region-capture"),
    (super::blur_default::COMMAND, "tool-action-new-blur"),
    (super::blur_numeric::COMMAND, "tool-action-selection-blur"),
    (super::numeric_style::MOSAIC, "tool-action-mosaic-cell"),
];
// Availability caches have fixed capacity; excess new catalog entries must fail at compile time rather than truncate high bits at runtime.
const _: () = assert!(ACTIONS.len() + super::tool_commands::TOOLS.len() <= u64::BITS as usize);
pub(super) fn label(command: usize, state: &State) -> Option<String> {
    let name = crate::ui_text::text(ACTIONS.iter().find(|entry| entry.0 == command)?.1);
    let style = state.canvas.borrow().editor.style();
    Some(match command {
        menu::WIDTH_DOWN | menu::WIDTH_UP => crate::ui_text::format(
            if command == menu::WIDTH_DOWN {
                "tool-action-width-down-value"
            } else {
                "tool-action-width-up-value"
            },
            &[("width", format!("{:.2}", style.width()).into())],
        ),
        0x4F => crate::ui_text::format(
            "tool-action-opacity-value",
            &[("opacity", format!("{:.0}", style.opacity() * 100.0).into())],
        ),
        0x46 => crate::ui_text::text(if style.fill().is_some() {
            "tool-action-fill-on"
        } else {
            "tool-action-fill-off"
        })
        .to_owned(),
        _ => name.to_owned(),
    })
}
// Owner drawing does not automatically update HWND text. Keep native names consistent with painted labels without repeated WM_SETTEXT.
pub(super) unsafe fn caption_matches(button: HWND, expected: &str) -> bool {
    let mut buffer = [0u16; 128];
    let length = GetWindowTextW(button, &mut buffer) as usize;
    buffer[..length].iter().copied().eq(expected.encode_utf16())
}
pub(super) unsafe fn sync_captions(panel: HWND, state: &State) -> Result<()> {
    for &(command, _) in ACTIONS {
        let button = GetDlgItem(Some(panel), command as i32)?;
        if let Some(text) = label(command, state) {
            if !caption_matches(button, &text) {
                SetWindowTextW(button, &HSTRING::from(text))?;
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_capture_button_creates_pin_instead_of_replacing_document() {
        assert_eq!(
            super::super::capture_input::DEFAULT,
            super::super::capture_input::NEW_PIN
        );
        assert!(ACTIONS
            .iter()
            .any(|entry| entry.0 == super::super::capture_input::DEFAULT
                && entry.1 == "tool-action-region-capture"));
        assert!(!ACTIONS
            .iter()
            .any(|entry| entry.0 == super::super::capture_input::COMMAND));
    }
    #[test]
    fn commands_are_unique_and_do_not_shadow_tools() {
        assert!(ACTIONS.len() + super::super::tool_commands::TOOLS.len() <= u64::BITS as usize);
        for (index, (command, label)) in ACTIONS.iter().enumerate() {
            assert!(!label.is_empty());
            assert!(!ACTIONS[..index].iter().any(|entry| entry.0 == *command));
            assert!(!super::super::tool_commands::TOOLS
                .iter()
                .any(|entry| entry.id == *command));
        }
    }
}
