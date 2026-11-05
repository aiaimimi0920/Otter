//! 会话级能力目录及原生加载菜单；失败保留旧目录，不自动加载或重试。
use super::{
    capability_job::{Job, Loaded, Source},
    State,
};
use windows::{
    core::{w, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) const COMMAND: usize = 0x1081;
#[derive(Default)]
pub(super) struct Catalog {
    pub requested: bool,
    pub job: Job,
    pub loaded: Option<std::rc::Rc<Loaded>>,
    pub failed: bool,
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let catalog = state.catalog.borrow();
    let label = if catalog.job.is_busy() {
        "正在加载节点能力…".into()
    } else if catalog.failed {
        "节点能力加载失败".into()
    } else if let Some(loaded) = &catalog.loaded {
        format!("节点能力：{}", loaded.handshake.arts().len())
    } else {
        "节点能力尚未加载".into()
    };
    AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, &HSTRING::from(label))?;
    AppendMenuW(
        menu,
        MF_STRING
            | if catalog.job.is_busy() {
                MF_GRAYED
            } else {
                MF_ENABLED
            },
        COMMAND,
        w!("加载节点能力"),
    )
}
pub(super) unsafe fn request(hwnd: HWND, state: &State) -> Result<()> {
    request_for(hwnd, &state.catalog)
}
pub(super) unsafe fn request_for(hwnd: HWND, catalog: &std::cell::RefCell<Catalog>) -> Result<()> {
    let mut catalog = catalog.borrow_mut();
    if catalog.job.is_busy() || catalog.requested {
        return Ok(());
    }
    catalog.requested = true;
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        catalog.requested = false;
        return Err(error);
    }
    Ok(())
}
impl super::application::Application {
    pub(super) fn process_catalog(&mut self) {
        let mut catalog = self.catalog.borrow_mut();
        if let Some(result) = catalog.job.poll() {
            match result {
                Ok(loaded) => {
                    catalog.loaded = Some(std::rc::Rc::new(loaded));
                    catalog.failed = false;
                }
                Err(_) => catalog.failed = true,
            }
        }
        if std::mem::take(&mut catalog.requested) && !catalog.job.is_busy() {
            catalog.failed = false;
            if catalog
                .job
                .start(
                    Source::Discovery(crate::loom_discovery::Config::from_env()),
                    self.execution_wake.callback(),
                )
                .is_err()
            {
                catalog.failed = true;
            }
        }
        drop(catalog);
        if let Err(error) = self.rebind_restored_arts() {
            eprintln!("native-session-rebind: {error}");
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_menu_wakeup_rolls_back_load_request() {
        let state = State::new(None, false).unwrap();
        assert!(unsafe { request(HWND(-2isize as *mut _), &state) }.is_err());
        assert!(!state.catalog.borrow().requested);
        assert!(!state.catalog.borrow().job.is_busy());
        assert!(super::super::group_lock::allows(COMMAND));
    }
}
