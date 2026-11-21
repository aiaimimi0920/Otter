//! 文档拥有的短暂扩展通知栈；主题只拥有表现/五秒回收，scope 撤权与队列预算留在 Otter。
use super::State;
use windows::{core::Result, Win32::Foundation::HWND};
struct Entry {
    scope: Option<String>,
    notice: neuro_mini_ui::Notice,
}
#[derive(Default)]
pub(super) struct Notices {
    entries: Vec<Entry>,
    stack: Option<neuro_mini_ui::NoticeStack>,
}
pub(super) unsafe fn show(
    hwnd: HWND,
    state: &State,
    scope: Option<&str>,
    title: &str,
    message: &str,
) -> Result<()> {
    let title: String = title
        .chars()
        .scan(0, |n, c| {
            *n += c.len_utf16();
            (*n <= 128).then_some(c)
        })
        .collect();
    let message: String = message
        .chars()
        .scan(0, |n, c| {
            *n += c.len_utf16();
            (*n <= 2048).then_some(c)
        })
        .collect();
    let mut notices = state.extension_notices.borrow_mut();
    if notices
        .stack
        .as_ref()
        .is_none_or(|stack| stack.window().is_none())
    {
        notices.entries.clear();
        notices.stack = Some(neuro_mini_ui::NoticeStack::new(hwnd)?);
    }
    let notice = notices
        .stack
        .as_ref()
        .unwrap()
        .push(&title, &message, "知道了")?;
    if notices.entries.len() >= 8 {
        notices.entries.pop();
    }
    notices.entries.insert(
        0,
        Entry {
            scope: scope.map(str::to_owned),
            notice,
        },
    );
    drop(notices);
    sync(state)
}
pub(super) unsafe fn sync(state: &State) -> Result<()> {
    let catalog = state.extensions.borrow();
    let mut notices = state.extension_notices.borrow_mut();
    notices.entries.retain(|n| {
        n.notice.window().is_some()
            && n.scope.as_ref().is_none_or(|scope| {
                catalog
                    .snapshot
                    .as_ref()
                    .is_some_and(|s| s.display_active(scope))
            })
    });
    if let Some(stack) = &notices.stack {
        stack.place()?;
    }
    Ok(())
}
pub(super) unsafe fn translate(
    state: &State,
    message: &windows::Win32::UI::WindowsAndMessaging::MSG,
) -> bool {
    state
        .extension_notices
        .borrow()
        .stack
        .as_ref()
        .is_some_and(|stack| stack.translate(message))
}
#[cfg(test)]
#[path = "extension_notices_tests.rs"]
mod tests;
