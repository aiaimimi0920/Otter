//! Compare the final menu tree passed to the owner-drawn host, preventing generic popups from silently adding pin features to the tray.
use super::*;
use crate::desktop::{context_menu, reference_ui as reference};
struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            DestroyMenu(self.0).unwrap();
        }
    }
}
#[test]
fn pin_tree_matches_main_order_and_only_adds_clipboard() {
    unsafe {
        let state = State::new(None, true).unwrap();
        let menu = Menu(CreatePopupMenu().unwrap());
        AppendMenuW(
            menu.0,
            MF_STRING,
            context_menu::EXPORT,
            &windows::core::HSTRING::from(crate::ui_text::text("common-save")),
        )
        .unwrap();
        reference::append_menu(menu.0, &state).unwrap();
        let tree = contents(&state, menu.0, false).unwrap().tree;
        let labels: Vec<_> = tree.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                crate::ui_text::text("common-save"),
                crate::ui_text::text("common-close"),
                crate::ui_text::text("library-recycle-bin"),
                crate::ui_text::text("library-empty-recycle-bin"),
                "",
                crate::ui_text::text("library-add-favorite"),
                crate::ui_text::text("library-favorites"),
                crate::ui_text::text("library-empty-favorites"),
                "",
                crate::ui_text::text("clipboard-history")
            ]
        );
        for i in [2, 6, 9] {
            assert!(tree[i].children.is_some());
        }
        for i in [4, 8] {
            assert!(tree[i].separator);
        }
        assert_eq!(tree[1].command, reference::RECYCLE);
        assert_eq!(tree[3].command, reference::CLEAR_RECYCLE);
        assert_eq!(tree[7].command, reference::CLEAR_REFERENCE);
        assert!(!tree[5].enabled); // Background indexing is incomplete; do not guess favorite status.
    }
}
#[test]
fn tray_tree_has_no_clipboard_libraries_or_hidden_restore_entry() {
    unsafe {
        let state = State::new(None, false).unwrap();
        let menu = Menu(CreatePopupMenu().unwrap());
        crate::desktop::tray::actions::append(menu.0, &state).unwrap();
        AppendMenuW(
            menu.0,
            MF_STRING,
            12,
            &windows::core::HSTRING::from(crate::ui_text::text("tray-autostart")),
        )
        .unwrap();
        AppendMenuW(
            menu.0,
            MF_STRING,
            2,
            &windows::core::HSTRING::from(crate::ui_text::text("tray-exit")),
        )
        .unwrap();
        let snapshot = contents(&state, menu.0, true).unwrap();
        let labels: Vec<_> = snapshot
            .tree
            .iter()
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(
            labels,
            [
                crate::ui_text::text("tray-capture"),
                crate::ui_text::text("tray-live-capture"),
                crate::ui_text::text("tray-long-capture"),
                crate::ui_text::text("tray-delay"),
                "",
                crate::ui_text::text("pin-pass-menu"),
                crate::ui_text::text("tray-receive"),
                crate::ui_text::text("tray-open-image"),
                crate::ui_text::text("tray-autostart"),
                crate::ui_text::text("tray-exit")
            ]
        );
        assert!(snapshot.history.is_none() && snapshot.frozen.is_none());
        assert!(snapshot
            .tree
            .iter()
            .enumerate()
            .all(|(i, item)| item.children.is_some() == matches!(i, 3 | 6)
                && item.separator == (i == 4)));
    }
}
