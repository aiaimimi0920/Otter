//! Document-owned transient extension-notice stacks; themes own appearance/five-second reclamation only, while scope revocation/queue budgets stay in Otter.
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
#[cfg(test)]
impl Notices {
    pub(super) fn live_unscoped_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.scope.is_none() && entry.notice.window().is_some())
            .count()
    }
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
    let notice = notices.stack.as_ref().unwrap().push(
        &title,
        &message,
        crate::ui_text::text("integration-acknowledge"),
    )?;
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "extension_notices_tests.rs"]
mod tests;
