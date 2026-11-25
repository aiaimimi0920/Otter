//! 原生弹出菜单的模态范围；退出嵌套消息循环后释放门禁，再由调用者执行命令。
use super::State;
use std::cell::Cell;
use windows::core::Result;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
#[path = "popup_history_deliver.rs"]
mod delivery;
#[path = "popup_frozen.rs"]
mod frozen;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
pub(in crate::desktop) use frozen::test_close_index_menu;
pub(in crate::desktop) use frozen::Workers;
#[path = "popup_history.rs"]
mod history;
#[path = "menu_target.rs"]
pub(super) mod target;
pub(super) enum Selection {
    Command(usize),
    History(crate::clipboard_history::Entry),
    Frozen(super::frozen_library_ui::Choice),
}
impl Default for Selection {
    fn default() -> Self {
        Self::Command(0)
    }
}

fn while_open<T: Default>(busy: &Cell<bool>, show: impl FnOnce() -> Result<T>) -> Result<T> {
    let Some(_modal) = super::file_dialog::Guard::begin(busy) else {
        return Ok(T::default());
    };
    show()
}
pub(super) unsafe fn track(
    hwnd: HWND,
    state: &State,
    menu: HMENU,
    point: POINT,
    current: impl FnOnce() -> bool,
) -> Result<usize> {
    let selection = while_open(&state.file_dialog, || {
        choose(hwnd, state, menu, point, false)
    })?;
    match selection {
        Selection::Command(command) => Ok(if IsWindow(Some(hwnd)).as_bool() && current() {
            command
        } else {
            0
        }),
        Selection::History(entry) => {
            deliver_history(hwnd, state, entry, current)?;
            Ok(0)
        }
        Selection::Frozen(choice) => {
            if IsWindow(Some(hwnd)).as_bool() && current() {
                super::frozen_library_ui::enqueue(hwnd, state, choice, point)?;
            }
            Ok(0)
        }
    }
}

pub(super) struct Contents {
    tree: Vec<crate::native_menu::Item>,
    history: Option<history::History>,
    frozen: Option<frozen::LibraryMenu>,
}
impl Contents {
    pub(super) fn update(
        &mut self,
        menu: &mut crate::native_menu::cascade::Cascade,
        mut remove: Option<&mut Remove<'_>>,
    ) -> Result<()> {
        if let Some(command) = menu.take_delete() {
            if !self
                .history
                .as_mut()
                .is_some_and(|history| history.delete(command))
            {
                if let (Some(frozen), Some(remove)) = (&mut self.frozen, &mut remove) {
                    frozen.remove(menu, Some(command), *remove)?;
                } else {
                    menu.finish_command(command);
                    return Ok(());
                }
            }
        }
        if let Some(history) = &mut self.history {
            history.update(menu)?;
        }
        if let Some(frozen) = &mut self.frozen {
            if let Some(remove) = &mut remove {
                frozen.remove(menu, None, *remove)?;
            }
            frozen.update(menu)?;
        }
        Ok(())
    }
}
pub(super) fn contents(state: &State, menu: HMENU, tray: bool) -> Result<Contents> {
    // HMENU 仍由业务构建器持有；自绘宿主只消费有界只读快照。
    // 失败不回退到另一个嵌套菜单，避免重放输入或绕过捕获/前台保护。
    let mut tree = crate::native_menu::snapshot(menu)?;
    let (source, workers) = {
        let view = state.frozen_library.borrow();
        let source = if view.busy { None } else { view.source.clone() };
        (source, view.workers.clone())
    };
    // 主干托盘没有剪贴板或冻结库；仅钉图菜单追加这些入口。
    let frozen = (!tray).then(|| {
        let mut frozen = frozen::LibraryMenu::attach(&mut tree, source, workers);
        frozen.bind_reference(
            &tree,
            &state.unit_id,
            !state.is_art() && !state.group_locked.get() && !state.frozen_library.borrow().busy,
        );
        frozen
    });
    let history = (!tray).then(|| {
        let index = history::History::attach(&mut tree);
        history::History::new(index)
    });
    Ok(Contents {
        tree,
        history,
        frozen,
    })
}
fn choose(hwnd: HWND, state: &State, menu: HMENU, point: POINT, tray: bool) -> Result<Selection> {
    choose_contents(hwnd, contents(state, menu, tray)?, point, tray, None)
}

pub(super) type Remove<'a> = dyn FnMut(Option<super::frozen_library_ui::Choice>) -> Result<Option<crate::session_source::Source>>
    + 'a;

pub(super) fn track_contents(
    hwnd: HWND,
    contents: Contents,
    point: POINT,
    busy: &Cell<bool>,
    remove: &mut Remove<'_>,
) -> Result<Selection> {
    while_open(busy, || {
        choose_contents(hwnd, contents, point, false, Some(remove))
    })
}

fn choose_contents(
    hwnd: HWND,
    mut contents: Contents,
    point: POINT,
    tray: bool,
    mut remove: Option<&mut Remove<'_>>,
) -> Result<Selection> {
    let tree = std::mem::take(&mut contents.tree);
    let update = |menu: &mut crate::native_menu::cascade::Cascade| {
        contents.update(menu, remove.as_deref_mut())
    };
    let command = if tray {
        crate::native_menu::cascade::track_tray_live(hwnd, tree, point, update)?
    } else {
        crate::native_menu::cascade::track_live(hwnd, tree, point, update)?
    }
    .unwrap_or(0);
    let selection = if history::History::is_history(command) {
        contents
            .history
            .as_mut()
            .and_then(|history| history.take(command))
            .map_or(Selection::Command(0), Selection::History)
    } else if let Some(choice) = contents
        .frozen
        .as_ref()
        .and_then(|frozen| frozen.take(command))
    {
        Selection::Frozen(choice)
    } else {
        Selection::Command(command)
    };
    Ok(selection)
}
#[cfg(test)]
pub(in crate::desktop) use frozen::remove_tests::exercise as test_continuous_removal;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(in crate::desktop) use frozen::smoke::select as smoke_frozen_selection;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(in crate::desktop) use frozen::smoke::select_action as smoke_frozen_action;

/// 菜单已关闭，且调用者仍须提供原目标凭证；冒烟测试与生产共用这条交付路径。
pub(in crate::desktop) unsafe fn deliver_history(
    hwnd: HWND,
    state: &State,
    entry: crate::clipboard_history::Entry,
    current: impl FnOnce() -> bool,
) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool() || state.file_dialog.get() || !current() {
        return Ok(());
    }
    delivery::deliver(hwnd, state, entry)
}

/// 托盘 owner 保持隐藏，只有临时菜单根面板参与前台激活。
pub(super) unsafe fn track_tray(
    hwnd: HWND,
    state: &State,
    menu: HMENU,
    point: POINT,
    deliver: impl FnOnce(crate::clipboard_history::Entry) -> Result<()>,
) -> Result<usize> {
    let target = target::MenuTarget::capture(&state.canvas.borrow());
    match while_open(&state.file_dialog, || {
        choose(hwnd, state, menu, point, true)
    })? {
        Selection::Command(command) => Ok(command),
        Selection::History(entry) => {
            deliver(entry)?;
            Ok(0)
        }
        Selection::Frozen(choice) => {
            if target.matches(&state.canvas.borrow()) {
                super::frozen_library_ui::enqueue(hwnd, state, choice, point)?;
            }
            Ok(0)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocks_nested_popup_and_releases_gate_before_returning_command() {
        let busy = Cell::new(false);
        assert_eq!(
            while_open(&busy, || {
                assert!(busy.get());
                assert_eq!(
                    while_open::<usize>(&busy, || panic!("nested popup")).unwrap(),
                    0
                );
                assert!(busy.get());
                Ok(7)
            })
            .unwrap(),
            7
        );
        assert!(!busy.get());
    }
    #[test]
    fn snapshot_failure_releases_gate_without_running_another_menu() {
        let busy = Cell::new(false);
        let result: Result<usize> = while_open(&busy, || {
            crate::native_menu::snapshot(HMENU::default())?;
            panic!("invalid menu must fail before command delivery")
        });
        assert!(result.is_err());
        assert!(!busy.get());
        assert_eq!(while_open(&busy, || Ok(0)).unwrap(), 0);
    }
}
#[cfg(test)]
#[path = "popup_menu_baseline_tests.rs"]
mod baseline_tests;
