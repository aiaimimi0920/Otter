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
            windows::core::w!("保存"),
        )
        .unwrap();
        reference::append_menu(menu.0, &state).unwrap();
        let tree = contents(&state, menu.0, false).unwrap().tree;
        let labels: Vec<_> = tree.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "保存",
                "关闭",
                "回收站",
                "清空回收站",
                "",
                "收藏",
                "收藏夹",
                "清空收藏夹",
                "",
                "剪贴板"
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
        AppendMenuW(menu.0, MF_STRING, 12, windows::core::w!("自启动")).unwrap();
        AppendMenuW(menu.0, MF_STRING, 2, windows::core::w!("退出")).unwrap();
        let snapshot = contents(&state, menu.0, true).unwrap();
        let labels: Vec<_> = snapshot
            .tree
            .iter()
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(
            labels,
            [
                "截图",
                "实时截图…",
                "长截图",
                "编辑已有图片…",
                "接收投射",
                "自启动",
                "退出"
            ]
        );
        assert!(snapshot.history.is_none() && snapshot.frozen.is_none());
        assert!(snapshot
            .tree
            .iter()
            .all(|item| item.children.is_none() && !item.separator));
    }
}
