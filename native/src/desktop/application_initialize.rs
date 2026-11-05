//! 应用初始化的唯一入口；从首窗口共享状态建立 owner，保持字段析构顺序不变。
use super::*;
use crate::desktop::{loom_execution_tasks, loom_input_tasks};

impl Application {
    pub fn new(instance: HINSTANCE, initial: DocumentWindow) -> Self {
        let hotkey_enabled = initial.state.hotkey_enabled.clone();
        let long_hotkey_enabled = initial.state.long_hotkey_enabled.clone();
        let live_hotkey_enabled = initial.state.live_hotkey_enabled.clone();
        let exit_requested = initial.state.exit_requested.clone();
        let session_end = initial.state.session_end.clone();
        let modals = initial.state.modals.clone();
        modals.register(&initial.state.file_dialog);
        let capture_active = initial.state.capture_active.clone();
        let capture_windows = initial.state.capture_windows.clone();
        let workspace_autosave = initial.state.workspace_autosave.clone();
        *capture_windows.borrow_mut() = vec![initial.window.0, initial.panel.0];
        let execution_wake = wake::Wake::new();
        let execution_jobs = loom_execution_tasks::Tasks::with_notify(execution_wake.callback());
        let session = save_context::fresh(initial.state.unit_id.clone());
        let chrome = super::chrome::Owner::default();
        *initial.state.chrome.borrow_mut() = chrome.bind();
        initial.state.frozen_library.borrow_mut().source = Some(session.original.clone());
        Self {
            chrome,
            capture_history: history::worker::Owner::new(
                initial.state.capture_history.clone(),
                execution_wake.callback(),
            ),
            history_window: None,
            authorization_owner: Default::default(),
            authorization: initial.state.authorization.clone(),
            live_sessions: Vec::new(),
            live_freeze_job: None,
            remote_sessions: Default::default(),
            surface_windows: Default::default(),
            live_visibility: Default::default(),
            display_cache: Default::default(),
            display_composition: Default::default(),
            parameter_file_job: None,
            naming_import_job: None,
            naming_save_job: None,
            naming_document: Default::default(),
            art_issuers: Default::default(),
            naming_source: None,
            naming_profile_target: None,
            naming_autosave: initial.state.naming_autosave.clone(),
            naming_autosave_request: false,
            catalog: initial.state.catalog.clone(),
            group_catalog: initial.state.group_catalog.clone(),
            workspace: crate::workspace::Workspace::with_initial(initial.state.unit_id.clone()),
            executions: crate::graph_requests::Requests::default(),
            execution_jobs,
            shader_installation: None,
            shader_cache: Default::default(),
            input_jobs: loom_input_tasks::Tasks::new(execution_wake.callback()),
            local_arts: super::super::art_local_owner::Owner::new(execution_wake.callback()),
            execution_wake,
            naming: initial.state.naming.clone(),
            cache_policy: initial.state.cache_policy.clone(),
            copy_authority: initial.state.copy_authority.clone(),
            editable_clipboard: initial.state.editable_clipboard.clone(),
            frozen_library: initial.state.frozen_library.clone(),
            documents: vec![initial],
            restored_session: Some(session),
            restore_job: None,
            save_job: None,
            save_origin: None,
            workspace_autosave,
            workspace_autosave_pending: None,
            workspace_checkpoint: None,
            projection_profile: Default::default(),
            projection_receive: Default::default(),
            projection_cleanup: Default::default(),
            projection_sender: Default::default(),
            projection_edit: Default::default(),
            recycle_maintenance_pending: false,
            recycle_policy: Default::default(),
            saved_state: None,
            exit_after_save: false,
            exit_saved: None,
            open_after_save: None,
            instance,
            error: None,
            hotkey_enabled,
            long_hotkey_enabled,
            live_hotkey_enabled,
            capture_active,
            capture_windows,
            modals,
            exit_requested,
            session_end,
        }
    }
}
