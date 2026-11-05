//! 应用级菜单命令路由；回调仅排队请求，生命周期操作由消息循环消费。
use super::*;

pub(in crate::desktop) unsafe fn request(hwnd: HWND, state: &State) -> Result<()> {
    if state.file_dialog.get() {
        return Ok(());
    }
    // 单个文档最多一个未处理请求，不随点击次数积累任务。
    if !state.new_window.replace(true) {
        if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
            state.new_window.set(false);
            return Err(error);
        }
    }
    Ok(())
}

pub(in crate::desktop) const NEW_WINDOW: usize = 0x104c;
pub(in crate::desktop) const EXIT_APPLICATION: usize = 0x104e;
pub(in crate::desktop) const SAVE_SESSION_AS: usize = 0x1059;
pub(in crate::desktop) const SAVE_SESSION: usize = 0x105a;
pub(in crate::desktop) const OPEN_SESSION: usize = 0x105b;
pub(in crate::desktop) const OPEN_SESSION_ASSETS: usize = 0x105c;
pub(in crate::desktop) const IMPORT_NAMING: usize = 0x105e;
pub(in crate::desktop) const SAVE_NAMING: usize = 0x105f;
pub(in crate::desktop) const UPDATE_NAMING: usize = 0x1060;
pub(in crate::desktop) const AUTO_NAMING: usize = 0x1061;
pub(in crate::desktop) const SAVE_PNG_DEFAULT: usize = 0x1062;
pub(in crate::desktop) const AUTO_WORKSPACE: usize = 0x106d;
pub(in crate::desktop) fn handles(command: usize) -> bool {
    if crate::desktop::reference_ui::handles(command) {
        return true;
    }
    if restore_batch::workflow::ui::handles(command) {
        return true;
    }
    if crate::desktop::device_authorization_ui::handles(command) {
        return true;
    }
    if command == crate::desktop::naming_input::COMMAND
        || command == crate::desktop::cache_input::COMMAND
    {
        return true;
    }
    matches!(
        command,
        NEW_WINDOW
            | EXIT_APPLICATION
            | SAVE_SESSION_AS
            | SAVE_SESSION
            | OPEN_SESSION
            | OPEN_SESSION_ASSETS
            | IMPORT_NAMING
            | SAVE_NAMING
            | UPDATE_NAMING
            | AUTO_NAMING
            | SAVE_PNG_DEFAULT
            | AUTO_WORKSPACE
    )
}
pub(in crate::desktop) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    if crate::desktop::reference_ui::handles(command) {
        return crate::desktop::reference_ui::request(hwnd, state, command);
    }
    if restore_batch::workflow::ui::handles(command) {
        return restore_batch::workflow::ui::request(hwnd, state, command);
    }
    if crate::desktop::device_authorization_ui::handles(command) {
        return crate::desktop::device_authorization_ui::execute(hwnd, state, command);
    }
    match command {
        crate::desktop::naming_input::COMMAND => crate::desktop::naming_input::edit(hwnd, state),
        crate::desktop::cache_input::COMMAND => crate::desktop::cache_input::edit(hwnd, state),
        NEW_WINDOW => request(hwnd, state),
        EXIT_APPLICATION => request_exit(hwnd, state),
        SAVE_SESSION_AS => save_ui::request(hwnd, state),
        SAVE_SESSION => save_ui::request_existing(hwnd, state),
        OPEN_SESSION => open_session::request(hwnd, state),
        OPEN_SESSION_ASSETS => open_session::request_assets(hwnd, state),
        IMPORT_NAMING => naming_import::request(hwnd, state),
        SAVE_NAMING => naming_save::request(hwnd, state),
        UPDATE_NAMING => naming_save::request_update(hwnd, state),
        SAVE_PNG_DEFAULT => {
            crate::desktop::export_job::start_default(hwnd, state);
            Ok(())
        }
        AUTO_NAMING => {
            if !state.modals.blocked() {
                state.naming_autosave.set(!state.naming_autosave.get());
            }
            Ok(())
        }
        AUTO_WORKSPACE => {
            if !state.modals.blocked() {
                state.workspace_autosave.toggle()?;
                PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0))?;
            }
            Ok(())
        }
        _ => Err(Error::new(E_INVALIDARG, "Unknown application command")),
    }
}
pub(in crate::desktop) unsafe fn append_menu(menu: HMENU, state: &State) -> Result<()> {
    crate::desktop::reference_ui::append_menu(menu, state)?;
    crate::desktop::device_authorization_ui::append_menu(menu, state)?;
    AppendMenuW(
        menu,
        MF_STRING,
        crate::desktop::cache_input::COMMAND,
        w!("临时缓存设置…"),
    )?;
    AppendMenuW(
        menu,
        MF_STRING
            | if state.resolved_display.borrow().is_some() || state.canvas.borrow().image.is_some()
            {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        SAVE_PNG_DEFAULT,
        w!("保存 PNG 到默认目录"),
    )?;
    AppendMenuW(
        menu,
        MF_STRING,
        crate::desktop::naming_input::COMMAND,
        w!("文件命名设置…"),
    )?;
    AppendMenuW(menu, MF_STRING, NEW_WINDOW, w!("新建工作窗口"))?;
    AppendMenuW(menu, MF_STRING, IMPORT_NAMING, w!("导入命名设置…"))?;
    AppendMenuW(menu, MF_STRING, SAVE_NAMING, w!("文件与缓存设置另存为…"))?;
    AppendMenuW(menu, MF_STRING, UPDATE_NAMING, w!("保存文件与缓存设置"))?;
    AppendMenuW(menu, MF_STRING, OPEN_SESSION, w!("打开工作区…"))?;
    AppendMenuW(
        menu,
        MF_STRING
            | if state.naming_autosave.get() {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
        AUTO_NAMING,
        w!("自动保存文件与缓存设置"),
    )?;
    AppendMenuW(
        menu,
        MF_STRING,
        OPEN_SESSION_ASSETS,
        w!("打开工作区（指定图片目录）…"),
    )?;
    AppendMenuW(menu, MF_STRING, SAVE_SESSION, w!("保存工作区"))?;
    AppendMenuW(menu, MF_STRING, SAVE_SESSION_AS, w!("工作区另存为…"))?;
    AppendMenuW(
        menu,
        MF_STRING
            | if state.workspace_autosave.enabled() {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            },
        AUTO_WORKSPACE,
        w!("自动保存当前工作区"),
    )?;
    restore_batch::workflow::ui::append_menu(menu)?;
    AppendMenuW(menu, MF_STRING, EXIT_APPLICATION, w!("退出 Otter"))
}
pub(in crate::desktop) unsafe fn request_exit(hwnd: HWND, state: &State) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(
            E_INVALIDARG,
            "Exit requester window is no longer valid",
        ));
    }
    if !state.exit_requested.replace(true) {
        // 应用请求使用线程唤醒；请求窗口随后销毁也不能把退出唤醒一并移除。
        if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
            state.exit_requested.set(false);
            return Err(error);
        }
    }
    Ok(())
}
