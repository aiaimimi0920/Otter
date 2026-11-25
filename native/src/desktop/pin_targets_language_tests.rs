//! Relabel the existing target picker without replacing LISTBOX selection or scroll state.
use super::*;
use crate::ui_text::{self, DisplayLanguage};
struct LanguageReset(DisplayLanguage);
impl Drop for LanguageReset {
    fn drop(&mut self) {
        ui_text::set_language(self.0);
    }
}
#[test]
fn language_refresh_keeps_target_identity_selection_and_scroll() {
    let _reset = LanguageReset(ui_text::current_language());
    ui_text::set_language(DisplayLanguage::English);
    unsafe {
        let owner = crate::desktop::Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                windows::core::w!("STATIC"),
                None,
                WS_POPUP,
                300,
                200,
                400,
                300,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let rows: Vec<_> = (0..20)
            .map(|index| Entry {
                id: format!("device-{index}"),
                name: format!("Device {index}"),
                checked: index == 12,
                enabled: true,
                command: index,
            })
            .collect();
        let menu = Window::open(
            owner.0,
            &rows,
            RECT {
                left: 320,
                top: 240,
                right: 360,
                bottom: 270,
            },
        )
        .unwrap();
        let hwnd = menu.hwnd();
        let list = GetDlgItem(Some(hwnd), LIST).unwrap();
        SendMessageW(list, LB_SETTOPINDEX, Some(WPARAM(8)), None);
        let top = SendMessageW(list, LB_GETTOPINDEX, None, None);
        let selection = SendMessageW(list, LB_GETSEL, Some(WPARAM(12)), None);
        ui_text::set_language(DisplayLanguage::SimplifiedChinese);
        menu.update(&rows, true, true, false);
        assert_eq!(menu.hwnd(), hwnd);
        assert_eq!(GetDlgItem(Some(hwnd), LIST).unwrap(), list);
        assert_eq!(SendMessageW(list, LB_GETTOPINDEX, None, None), top);
        assert_eq!(
            SendMessageW(list, LB_GETSEL, Some(WPARAM(12)), None),
            selection
        );
        let button = GetDlgItem(Some(hwnd), 3).unwrap();
        let mut text = [0u16; 64];
        let length = GetWindowTextW(button, &mut text) as usize;
        assert_eq!(String::from_utf16_lossy(&text[..length]), "刷新");
        assert!(menu.event().is_none());
    }
}
