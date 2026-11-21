//! Application 持有菜单循环；消息间只推进本菜单的删除，不重入通用业务泵。
use super::*;
use crate::desktop::{context_menu as ui, popup_menu as popup};
use std::cell::Cell;

// 仅在不泵消息的事务准入/发布期间暂停本菜单门禁；其它窗口的模态仍生效。
struct Transaction<'a>(&'a Cell<bool>);
impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        self.0.set(true);
    }
}
#[derive(Default)]
struct Removal {
    active: bool,
}
impl Removal {
    unsafe fn step(
        &mut self,
        app: &mut Application,
        pending: &ui::Pending,
        flag: &Cell<bool>,
        choice: Option<crate::desktop::frozen_library_ui::Choice>,
    ) -> Result<Option<crate::session_source::Source>> {
        if choice.is_none() && !self.active {
            return Ok(None);
        }
        if !flag.get() || !current(app, pending) {
            if self.active {
                if let Some((_, job)) = &app.save_job {
                    job.cancel();
                }
            }
            return Err(Error::new(E_ABORT, "Menu removal target changed"));
        }
        if self.active && !app.save_job.as_ref().is_some_and(|(_, job)| job.is_ready()) {
            return Ok(None);
        }
        let result = {
            flag.set(false);
            let _transaction = Transaction(flag);
            if let Some(choice) = choice {
                if self.active {
                    return Err(Error::new(E_ABORT, "Menu removal already active"));
                }
                app.request_frozen_remove(choice).map(|()| {
                    self.active = true;
                    None
                })
            } else {
                app.poll_reference().map(|()| {
                    self.active = false;
                    Some(app.restored_session.as_ref().unwrap().original.clone())
                })
            }
        };
        app.sync_frozen_menu();
        if !current(app, pending) {
            if self.active {
                if let Some((_, job)) = &app.save_job {
                    job.cancel();
                }
            }
            return Err(Error::new(E_ABORT, "Menu removal owner changed"));
        }
        result
    }
}
unsafe fn current(app: &Application, pending: &ui::Pending) -> bool {
    app.documents.iter().any(|doc| {
        doc.state.unit_id == pending.origin
            && doc.is_alive()
            && pending.target.matches(&doc.state.canvas.borrow())
    })
}
impl Application {
    pub(super) unsafe fn process_context_menu(&mut self) {
        let pending = self.frozen_library.borrow_mut().menu.take();
        let Some(pending) = pending else { return };
        if let Err(error) = self.show_context_menu(&pending) {
            eprintln!("native-menu: {error}");
            if let Some(doc) = self
                .documents
                .iter()
                .find(|doc| doc.state.unit_id == pending.origin && doc.is_alive())
            {
                MessageBoxW(
                    Some(doc.window.0),
                    &HSTRING::from(format!("无法完成菜单操作，未提交的修改已保留。\n{error}")),
                    w!("Otter"),
                    MB_OK | MB_ICONWARNING,
                );
            }
        }
    }
    unsafe fn show_context_menu(&mut self, pending: &ui::Pending) -> Result<()> {
        if !current(self, pending) || self.modals.blocked() {
            return Ok(());
        }
        // 独立快照和 Rc 门禁不借用 documents，后台完成可以安全回到 Application。
        let (hwnd, flag, contents) = {
            let doc = self
                .documents
                .iter()
                .find(|doc| doc.state.unit_id == pending.origin)
                .unwrap();
            let menu = ui::build(&doc.state)?;
            (
                doc.window.0,
                doc.state.file_dialog.clone(),
                popup::contents(&doc.state, menu.0, false)?,
            )
        };
        let mut removal = Removal::default();
        let selected =
            popup::track_contents(hwnd, contents, pending.point, &flag, &mut |choice| {
                removal.step(self, pending, &flag, choice)
            })?;
        if !current(self, pending) {
            return Ok(());
        }
        let state = &self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == pending.origin)
            .unwrap()
            .state;
        match selected {
            popup::Selection::Command(command) => ui::execute(hwnd, state, command),
            popup::Selection::History(entry) => popup::deliver_history(hwnd, state, entry, || {
                pending.target.matches(&state.canvas.borrow())
            }),
            popup::Selection::Frozen(choice) => {
                crate::desktop::frozen_library_ui::enqueue(hwnd, state, choice, pending.point)
            }
        }
    }
}

#[cfg(test)]
#[path = "application_context_menu_tests.rs"]
mod tests;
