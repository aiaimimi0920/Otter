//! 钉图中继开关只修改文档策略；依赖凭证在主循环退休旧准备/执行，不在菜单中等待。
use super::State;
use windows::{
    core::{w, Result},
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::*,
    },
};
pub(super) const COMMAND: usize = 0x1084;
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    if state.is_art() {
        return Ok(());
    }
    AppendMenuW(
        menu,
        MF_STRING
            | if state.accept_upstream.get() {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            }
            | if state.group_locked.get() {
                MF_GRAYED
            } else {
                MF_ENABLED
            },
        COMMAND,
        w!("接收上游图片"),
    )
}
pub(super) unsafe fn toggle(hwnd: HWND, state: &State) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool() || state.group_locked.get() || state.is_art() {
        return Ok(());
    }
    let before = state.accept_upstream.get();
    state.accept_upstream.set(!before);
    if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
        state.accept_upstream.set(before);
        return Err(error);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Window(HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }
    struct Menu(HMENU);
    impl Drop for Menu {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyMenu(self.0);
            }
        }
    }
    #[test]
    fn menu_toggle_invalidates_dependencies_and_respects_window_and_group_lifetime() {
        let state = State::new(None, false).unwrap();
        let stamp = crate::desktop::configuration_stamp::Stamp::capture(&state);
        unsafe {
            let window = Window(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("relay test"),
                    WINDOW_STYLE::default(),
                    0,
                    0,
                    1,
                    1,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            let menu = Menu(CreatePopupMenu().unwrap());
            append(menu.0, &state).unwrap();
            assert_ne!(
                GetMenuState(menu.0, COMMAND as u32, MF_BYCOMMAND) & MF_CHECKED.0,
                0
            );
            state.group_locked.set(true);
            toggle(window.0, &state).unwrap();
            assert!(stamp.matches(&state));
            state.group_locked.set(false);
            crate::desktop::context_menu::execute(window.0, &state, COMMAND).unwrap();
            assert!(!state.accept_upstream.get());
            assert!(!stamp.matches(&state));
            let menu = Menu(CreatePopupMenu().unwrap());
            append(menu.0, &state).unwrap();
            assert_eq!(
                GetMenuState(menu.0, COMMAND as u32, MF_BYCOMMAND) & MF_CHECKED.0,
                0
            );
            toggle(HWND::default(), &state).unwrap();
            assert!(!state.accept_upstream.get());
        }
    }
}
