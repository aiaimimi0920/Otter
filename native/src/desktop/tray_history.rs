//! 托盘历史复用菜单交付；仅文字选择恢复原窗口，恢复之后再次验证原目标。
use super::*;
use crate::{
    clipboard_history::{Content, Entry},
    desktop::popup_menu::{self, target::MenuTarget},
};
use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;

fn route(
    text: bool,
    ready: impl Fn() -> bool,
    restore: impl FnOnce(),
    deliver: impl FnOnce() -> Result<()>,
) -> Result<()> {
    if !ready() {
        return Ok(());
    }
    if text {
        restore();
    }
    if ready() {
        deliver()?;
    }
    Ok(())
}

pub(super) unsafe fn deliver(
    hwnd: HWND,
    state: &State,
    entry: Entry,
    target: &MenuTarget,
) -> Result<()> {
    let text = matches!(&entry.content, Content::Text(_));
    route(
        text,
        || {
            IsWindow(Some(hwnd)).as_bool()
                && IsWindowEnabled(hwnd).as_bool()
                && !state.file_dialog.get()
                && target.matches(&state.canvas.borrow())
        },
        || super::restore_window(hwnd, state),
        || {
            popup_menu::deliver_history(hwnd, state, entry, || {
                target.matches(&state.canvas.borrow())
            })
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn images_do_not_restore_and_text_rechecks_after_restore() {
        let calls = std::cell::RefCell::new(Vec::new());
        for text in [false, true] {
            calls.borrow_mut().clear();
            route(
                text,
                || true,
                || calls.borrow_mut().push("restore"),
                || {
                    calls.borrow_mut().push("deliver");
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(
                *calls.borrow(),
                if text {
                    vec!["restore", "deliver"]
                } else {
                    vec!["deliver"]
                }
            );
        }
        let valid = Cell::new(true);
        route(
            true,
            || valid.get(),
            || valid.set(false),
            || panic!("stale target after restore"),
        )
        .unwrap();
        route(
            true,
            || false,
            || panic!("busy restore"),
            || panic!("busy delivery"),
        )
        .unwrap();
    }
    #[test]
    fn delivery_error_is_not_replayed_or_converted_to_success() {
        assert!(route(
            false,
            || true,
            || panic!("image restore"),
            || Err(Error::new(E_FAIL, "synthetic failure"))
        )
        .is_err());
    }
}
