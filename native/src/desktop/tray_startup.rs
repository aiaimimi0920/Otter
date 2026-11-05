//! 托盘自启动选项；只在用户选择后更新，按菜单快照拒绝状态已变化的操作。
use super::*;
use crate::autostart::{self, Status};
pub(super) const COMMAND: usize = 0x106a;
fn presentation(status: Option<Status>) -> (MENU_ITEM_FLAGS, &'static str) {
    match status {
        Some(Status::Enabled) => (MF_STRING | MF_CHECKED, "自启动"),
        Some(Status::Disabled) => (MF_STRING, "自启动"),
        Some(Status::OtherInstallation) => (MF_STRING | MF_GRAYED, "自启动（其他 Otter 安装）"),
        None => (MF_STRING | MF_GRAYED, "自启动（读取失败）"),
    }
}
pub(super) unsafe fn append(menu: HMENU) -> Result<Option<Status>> {
    let status = match autostart::query() {
        Ok(status) => Some(status),
        Err(error) => {
            eprintln!("native-autostart-read: {error}");
            None
        }
    };
    let (flags, label) = presentation(status);
    AppendMenuW(menu, flags, COMMAND, &windows::core::HSTRING::from(label))?;
    Ok(status)
}
fn update(
    snapshot: Option<Status>,
    query: impl FnOnce() -> std::io::Result<Status>,
    set: impl FnOnce(bool) -> std::io::Result<Status>,
) -> std::io::Result<()> {
    let Some(expected @ (Status::Enabled | Status::Disabled)) = snapshot else {
        return Err(std::io::Error::other("Startup state is unavailable"));
    };
    if query()? != expected {
        return Err(std::io::Error::other(
            "Startup state changed while menu was open",
        ));
    }
    set(expected == Status::Disabled)?;
    Ok(())
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, snapshot: Option<Status>) {
    if state.file_dialog.get() || state.modals.blocked() {
        return;
    }
    if let Err(error) = update(snapshot, autostart::query, autostart::set_enabled) {
        eprintln!("native-autostart-update: {error}");
        let Some(_guard) = crate::desktop::file_dialog::Guard::begin(&state.file_dialog) else {
            return;
        };
        MessageBoxW(
            Some(hwnd),
            w!("无法更新自启动设置。启动项可能已变化或当前用户权限不足，请重新打开托盘菜单检查。"),
            w!("Otter"),
            MB_OK | MB_ICONERROR,
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_state_is_actual_and_unknown_or_foreign_entries_are_disabled() {
        assert_eq!(
            presentation(Some(Status::Enabled)).0 & MF_CHECKED,
            MF_CHECKED
        );
        for status in [None, Some(Status::OtherInstallation)] {
            assert_eq!(presentation(status).0 & MF_GRAYED, MF_GRAYED);
            assert!(update(
                status,
                || panic!("unknown read"),
                |_| panic!("unknown write")
            )
            .is_err());
        }
        assert!(update(
            Some(Status::Disabled),
            || Ok(Status::Enabled),
            |_| panic!("stale write")
        )
        .is_err());
        update(
            Some(Status::Enabled),
            || Ok(Status::Enabled),
            |enabled| {
                assert!(!enabled);
                Ok(Status::Disabled)
            },
        )
        .unwrap();
        update(
            Some(Status::Disabled),
            || Ok(Status::Disabled),
            |enabled| {
                assert!(enabled);
                Ok(Status::Enabled)
            },
        )
        .unwrap();
    }
}
