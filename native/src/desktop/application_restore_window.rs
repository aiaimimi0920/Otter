//! 恢复窗口只准备隐藏 owner；共享应用控制状态，但不发布文档、分组或工作区变化。
use super::*;

impl Application {
    pub(super) unsafe fn prepare_restored_window(
        &self,
        state: State,
        physical: crate::pin_geometry::PinRect,
    ) -> Result<DocumentWindow> {
        if self.modals.blocked() || self.exit_requested.get() {
            return Err(Error::new(
                E_ABORT,
                "Application cannot prepare restored window now",
            ));
        }
        DocumentWindow::create_at(self.instance, self.bind_window_state(state), physical)
    }
}

pub(super) unsafe fn smoke(app: &Application) -> Result<()> {
    let count = app.documents.len();
    let unit_count = app.workspace.units().len();
    let physical = crate::pin_geometry::PinRect {
        x: -100,
        y: 20,
        width: 240,
        height: 160,
    };
    let state = State::new(None, true)?;
    let unit = state.unit_id.clone();
    let document = app.prepare_restored_window(state, physical)?;
    let hwnd = document.window.0;
    if !document.is_alive()
        || IsWindowVisible(hwnd).as_bool()
        || app.workspace.contains(&unit)
        || app.source_window(&unit).is_some()
        || !std::rc::Rc::ptr_eq(&document.state.catalog, &app.catalog)
        || !std::rc::Rc::ptr_eq(&document.state.group_catalog, &app.group_catalog)
        || !std::rc::Rc::ptr_eq(&document.state.hotkey_enabled, &app.hotkey_enabled)
        || !std::rc::Rc::ptr_eq(
            &document.state.long_hotkey_enabled,
            &app.long_hotkey_enabled,
        )
        || !std::rc::Rc::ptr_eq(
            &document.state.live_hotkey_enabled,
            &app.live_hotkey_enabled,
        )
        || !std::rc::Rc::ptr_eq(&document.state.exit_requested, &app.exit_requested)
        || !std::rc::Rc::ptr_eq(&document.state.capture_active, &app.capture_active)
        || !std::rc::Rc::ptr_eq(&document.state.capture_windows, &app.capture_windows)
        || document.state.global_capture.get()
        || document.state.global_long_capture.get()
        || document.state.global_live_capture.get()
    {
        return Err(Error::new(
            E_FAIL,
            "Restored window shared state or publication mismatch",
        ));
    }
    // 模态登记仅持有弱引用；候选释放后不应留下阻塞或隐式活动窗口。
    document.state.file_dialog.set(true);
    if !app.modals.blocked() {
        return Err(Error::new(
            E_FAIL,
            "Restored window modal state is not shared",
        ));
    }
    if app
        .prepare_restored_window(State::new(None, true)?, physical)
        .is_ok()
    {
        return Err(Error::new(
            E_FAIL,
            "Modal application admitted restored window",
        ));
    }
    document.state.file_dialog.set(false);
    drop(document);
    if IsWindow(Some(hwnd)).as_bool()
        || app.modals.blocked()
        || app.documents.len() != count
        || app.workspace.units().len() != unit_count
    {
        return Err(Error::new(
            E_FAIL,
            "Restored candidate cleanup changed application",
        ));
    }
    let invalid = crate::pin_geometry::PinRect {
        width: 0,
        ..physical
    };
    if app
        .prepare_restored_window(State::new(None, true)?, invalid)
        .is_ok()
        || app.modals.blocked()
        || app.documents.len() != count
    {
        return Err(Error::new(
            E_FAIL,
            "Failed restored window leaked application state",
        ));
    }
    println!(
        "native-restored-application-smoke: shared=true hidden=true unpublished=true cleanup=true"
    );
    Ok(())
}
