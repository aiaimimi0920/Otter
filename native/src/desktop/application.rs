//! Single-UI-thread document collection; execute creation requests after callbacks return to avoid reentrant window creation while borrowing the model.
use super::{document_window::DocumentWindow, State};
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

#[path = "application_chrome.rs"]
pub(super) mod chrome;

#[path = "application_account_owner.rs"]
mod account_owner;
#[path = "application_account_projection.rs"]
mod account_projection;
#[path = "application_add_art.rs"]
mod add_art;
#[path = "application_authorization.rs"]
mod authorization;
#[path = "application_commands.rs"]
mod commands;
#[path = "application_context_menu.rs"]
mod context_menu;
#[path = "application_frozen_menu.rs"]
mod frozen_menu;
#[path = "application_loom_connection.rs"]
mod loom_connection;
#[path = "application_projection_inbox.rs"]
mod projection;
#[path = "application_projection_edit.rs"]
mod projection_edit;
#[path = "application_projection_sender.rs"]
mod projection_sender;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(super) use commands::request;
pub(super) use commands::{
    execute, handles, request_exit, AUTO_NAMING, EXIT_APPLICATION, IMPORT_NAMING, NEW_WINDOW,
    OPEN_SESSION, OPEN_SESSION_ASSETS, SAVE_NAMING, SAVE_PNG_DEFAULT, SAVE_SESSION,
    SAVE_SESSION_AS, UPDATE_NAMING,
};
#[path = "application_art_issuer_confirmation.rs"]
mod art_issuer_confirmation;
#[cfg(test)]
#[path = "application_chain_tests.rs"]
mod chain_tests;
#[path = "application_display.rs"]
mod display;
#[path = "application_display_composition.rs"]
mod display_composition;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_drop_smoke.rs"]
mod drop_smoke;
#[path = "application_edit_palette.rs"]
mod edit_palette;
#[path = "application_edit_retirement.rs"]
mod edit_retirement;
#[path = "application_execution.rs"]
mod execution;
#[path = "application_execution_start.rs"]
mod execution_start;
#[cfg(test)]
#[path = "application_execution_test_peer.rs"]
mod execution_test_peer;
#[cfg(test)]
#[path = "application_execution_test_pump.rs"]
mod execution_test_pump;
#[path = "application_exit.rs"]
mod exit;
#[path = "application_exit_intent.rs"]
mod exit_intent;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_exit_save_smoke.rs"]
mod exit_save_smoke;
#[cfg(test)]
#[path = "application_extension_catalog_tests.rs"]
mod extension_catalog_tests;
#[path = "application_extension_shortcuts.rs"]
mod extension_shortcuts;
#[path = "application_extension_views.rs"]
mod extension_views;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_group_lock_smoke.rs"]
mod group_lock_smoke;
#[path = "application_group_visibility.rs"]
mod group_visibility;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_group_visibility_smoke.rs"]
mod group_visibility_smoke;
#[path = "application_groups.rs"]
mod groups;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_groups_smoke.rs"]
mod groups_smoke;
#[path = "application_capture_history.rs"]
mod history;
#[path = "application_image_inputs.rs"]
mod image_inputs;
#[path = "application_import.rs"]
mod import;
#[path = "application_initialize.rs"]
mod initialize;
#[path = "application_input_preparation.rs"]
mod input_preparation;
#[path = "application_links.rs"]
mod links;
#[path = "application_live.rs"]
mod live;
#[path = "application_live_viewer.rs"]
mod live_viewer;
#[path = "application_local_art.rs"]
mod local_art;
#[path = "application_local_art_node.rs"]
mod local_art_node;
#[path = "application_locale.rs"]
pub(super) mod locale;
#[path = "application_locale_remote.rs"]
mod locale_remote;
#[path = "application_loop.rs"]
mod message_loop;
#[path = "application_naming_autosave.rs"]
mod naming_autosave;
#[path = "application_naming_import.rs"]
mod naming_import;
#[path = "application_naming_save.rs"]
mod naming_save;
#[path = "application_open_session.rs"]
mod open_session;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_open_smoke.rs"]
mod open_smoke;
#[path = "application_parameter_files.rs"]
mod parameter_files;
#[path = "application_parameter_panel.rs"]
mod parameter_panel;
#[path = "application_parameter_views.rs"]
mod parameter_views;
#[path = "application_parameters.rs"]
mod parameters;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_paste_smoke.rs"]
mod paste_smoke;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_pin_smoke.rs"]
mod pin_smoke;
#[path = "application_propagation.rs"]
mod propagation;
#[path = "application_remote_live.rs"]
mod remote_live;
#[path = "application_resident.rs"]
mod resident;
#[path = "application_restore_batch.rs"]
pub mod restore_batch;
#[path = "application_restore_rebind.rs"]
mod restore_rebind;
#[path = "application_restore_window.rs"]
mod restore_window;
#[path = "application_save_context.rs"]
mod save_context;
#[path = "application_save_control.rs"]
mod save_control;
#[path = "application_save_disk.rs"]
mod save_disk;
#[path = "application_save_publish.rs"]
mod save_publish;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_save_replace_smoke.rs"]
mod save_replace_smoke;
#[path = "application_save_snapshot.rs"]
pub mod save_snapshot;
#[path = "application_save_state.rs"]
mod save_state;
#[path = "application_save_ui.rs"]
mod save_ui;
#[path = "application_session_end.rs"]
mod session_end;
#[path = "application_shader_startup.rs"]
mod shader_startup;
#[path = "application_shutdown.rs"]
mod shutdown;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_smoke.rs"]
mod smoke;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_smoke_session_fixture.rs"]
mod smoke_session_fixture;
#[path = "application_surface_admission.rs"]
mod surface_admission;
#[path = "application_surface_confirmation.rs"]
mod surface_confirmation;
#[path = "application_surface_recovery.rs"]
mod surface_recovery;
#[path = "application_surface_remount.rs"]
mod surface_remount;
#[path = "application_surface_start.rs"]
mod surface_start;
#[path = "application_wake.rs"]
mod wake;
#[path = "application_window_state.rs"]
mod window_state;
#[path = "application_workspace_autosave.rs"]
pub(super) mod workspace_autosave;
#[path = "application_workspace_checkpoint.rs"]
mod workspace_checkpoint;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(super) use smoke::run as smoke;

pub(super) struct Application {
    chrome: chrome::Owner,
    capture_history: history::worker::Owner,
    edit_palette: super::edit_palette::Shared,
    history_window: Option<history::window::Window>,
    authorization_owner: crate::device_authorization_owner::Owner,
    local_loom: crate::loom_local_connection::owner::Owner,
    authorization: std::rc::Rc<std::cell::RefCell<super::device_authorization_ui::View>>,
    live_sessions: Vec<super::composition::live::session::Session>,
    live_freeze_job: Option<live::freeze::Pending>,
    remote_sessions: super::composition::relay::sessions::Sessions,
    surface_windows: super::surface_windows::Windows,
    live_visibility: crate::live_visibility_owner::Owner,
    display_cache: std::cell::RefCell<Option<display::Cache>>,
    workspace: crate::workspace::Workspace,
    executions: crate::graph_requests::Requests,
    // Fields drop in declaration order: even abnormal exits stop notifications before reaping background tasks.
    pub(super) execution_wake: wake::Wake,
    extensions: super::extension_catalog::Owner,
    extension_commands: super::extension_commands::Owner,
    extension_shortcuts: super::extension_shortcuts::Owner,
    display_composition: std::cell::RefCell<display_composition::Cache>,
    parameter_file_job: Option<super::parameter_file_job::Job>,
    naming_import_job: Option<naming_import::Pending>,
    naming_save_job: Option<naming_save::Pending>,
    locale: locale::Owner,
    locale_remote: locale_remote::Owner,
    naming_document: std::sync::Arc<crate::app_settings_document::Document>,
    art_issuers: crate::art_issuer_trust::Authority,
    naming_source: Option<std::sync::Arc<crate::settings_source::Source>>,
    naming_profile_target: Option<std::path::PathBuf>,
    naming_autosave: std::rc::Rc<std::cell::Cell<bool>>,
    naming_autosave_request: bool,
    pub(super) catalog: std::rc::Rc<std::cell::RefCell<super::capability_catalog::Catalog>>,
    input_jobs: super::loom_input_tasks::Tasks,
    local_arts: super::art_local_owner::Owner,
    execution_jobs: super::loom_execution_tasks::Tasks,
    shader_installation: Option<super::shader_prepare::Installation>,
    shader_cache: super::shader_recipe_cache::Cache,
    group_catalog: std::rc::Rc<std::cell::RefCell<Vec<crate::workspace::Group>>>,
    documents: Vec<DocumentWindow>,
    naming: std::rc::Rc<std::cell::RefCell<crate::file_naming_settings::Settings>>,
    cache_policy: std::rc::Rc<std::cell::Cell<crate::cache_policy::Policy>>,
    copy_authority: crate::desktop::copy_job::Authority,
    editable_clipboard: crate::desktop::clipboard_editable::Shared,
    restored_session: Option<restore_batch::Record>,
    retired_workflow: Option<restore_batch::workflow::retirement::Jobs>,
    retired_edits: edit_retirement::Retired,
    frozen_library: std::rc::Rc<std::cell::RefCell<super::frozen_library_ui::View>>,
    restore_job: Option<restore_batch::Job>,
    save_job: Option<(save_snapshot::Receipt, save_snapshot::job::Job)>,
    save_origin: Option<crate::workspace_id::UnitId>,
    workspace_autosave: std::rc::Rc<workspace_autosave::Control>,
    workspace_autosave_pending: Option<u64>,
    workspace_checkpoint: Option<workspace_checkpoint::Owner>,
    projection_profile: projection::profile::Owner,
    projection_receive: projection::receive::Owner,
    projection_cleanup: projection::cleanup::Owner,
    projection_sender: projection_sender::Owner,
    account_projection: account_owner::Owner,
    projection_edit: projection_edit::Owner,
    recycle_maintenance_pending: bool,
    recycle_policy: crate::session_library::recycle::policy::Policy,
    saved_state: Option<save_state::Stamp>,
    exit_after_save: bool,
    exit_saved: Option<exit_intent::Intent>,
    open_after_save: Option<open_session::Target>,
    instance: HINSTANCE,
    error: Option<Error>,
    hotkey_enabled: std::rc::Rc<std::cell::Cell<bool>>,
    long_hotkey_enabled: std::rc::Rc<std::cell::Cell<bool>>,
    live_hotkey_enabled: std::rc::Rc<std::cell::Cell<bool>>,
    capture_active: std::rc::Rc<std::cell::Cell<bool>>,
    capture_windows: std::rc::Rc<std::cell::RefCell<Vec<HWND>>>,
    modals: super::modal_scope::Scope,
    exit_requested: std::rc::Rc<std::cell::Cell<bool>>,
    exit_committed: bool,
    session_end: std::rc::Rc<super::session_end::Control>,
}

impl Application {
    unsafe fn collect_closed(&mut self) {
        self.retired_edits.reap();
        self.frozen_library.borrow().workers.reap();
        for doc in &self.documents {
            if doc.state.live_pin.get() && !doc.is_alive() {
                if let Some(session) = self
                    .live_sessions
                    .iter_mut()
                    .find(|s| s.unit_id() == &doc.state.unit_id)
                {
                    session.stop();
                }
            }
        }
        self.collect_deleted_projections();
        self.collect_deleted_senders();
        let previous = self.documents.len();
        self.documents.retain(|document| {
            if document.is_alive() {
                return true;
            }
            if self.error.is_none() {
                self.error = document.state.error.borrow_mut().take();
            }
            self.retired_edits.take_from(&document.state);
            false
        });
        if self.documents.len() != previous {
            self.retain_workspace_owners();
            if self.workspace.units().is_empty() {
                self.workspace.clear();
            }
            self.executions
                .retain_units(|id| self.workspace.contains(id));
            self.execution_jobs
                .retain_units(|id| self.workspace.contains(id));
            self.local_arts
                .retire(&self.documents, &self.workspace, &self.live_sessions);
            self.sync_capture_windows();
            self.sync_groups();
        }
    }

    unsafe fn pending_windows(&mut self, visible: bool) -> Result<()> {
        // Acknowledge old notifications before reading the mailbox or starting new tasks, so fast completion is not merged into an old pending wakeup and lost.
        self.execution_wake.acknowledge();
        self.settle_edit_palette();
        self.process_capture_history(visible);
        self.process_restore(visible)?;
        self.sync_frozen_menu();
        // Create new HWNDs only after reaping old owners, so handle reuse cannot overlap an old owner's destruction.
        self.collect_closed();
        self.process_live(visible)?;
        self.process_frozen_menu(visible)?;
        self.process_authorization();
        self.process_remote_locale();
        for doc in &self.documents {
            if super::display_edit::settle(&doc.state) {
                let _ =
                    windows::Win32::Graphics::Gdi::InvalidateRect(Some(doc.window.0), None, false);
            }
        }
        self.process_catalog();
        self.sync_extension_views();
        self.process_art_requests(visible)?;
        self.process_workflow_sync();
        self.pending_link_changes();
        self.pending_parameter_files(visible)?;
        self.cancel_requested_executions();
        self.process_local_arts(visible);
        self.input_jobs
            .retain_units(|unit| self.workspace.contains(unit));
        self.input_jobs.poll();
        self.retire_preparations();
        self.collect_prepared_inputs();
        self.execution_jobs.retire_stale(&self.executions);
        self.execution_jobs.retire_changed(|unit, stamp| {
            stamp.matches_owners(unit, &self.documents, &self.live_sessions, &self.workspace)
        });
        self.execution_jobs.poll();
        self.collect_execution_updates();
        self.collect_execution_results();
        self.sync_display_frames();
        self.process_parameter_panels(visible);
        self.sync_workspace_chrome(visible);
        self.start_pending_executions();
        // Collect/reap existing tasks before scheduling the retained latest request; even the final completion notification can advance the queue.
        self.start_requested_preparations(visible);
        let requests: Vec<_> = self
            .documents
            .iter()
            .filter(|document| document.state.new_window.replace(false))
            .map(|document| document.state.unit_id.clone())
            .collect();
        for source in requests {
            let result = self.add_window(visible);
            if let Err(error) = result {
                if !visible {
                    return Err(error);
                }
                if let Some(source) = self.source_window(&source) {
                    MessageBoxW(
                        Some(source),
                        &HSTRING::from(crate::ui_text::format(
                            "action-new-window-failed",
                            &[("error", error.to_string().into())],
                        )),
                        w!("Otter"),
                        MB_OK | MB_ICONWARNING,
                    );
                }
            }
        }
        let pins: Vec<_> = self
            .documents
            .iter()
            .filter_map(|document| {
                document
                    .state
                    .new_pin
                    .borrow_mut()
                    .take()
                    .map(|pin| (document.state.unit_id.clone(), pin))
            })
            .collect();
        for (source, pins) in pins {
            let result = self.add_prepared(pins, visible);
            if let Err(error) = result {
                if !visible {
                    return Err(error);
                }
                if let Some(source) = self.source_window(&source) {
                    MessageBoxW(
                        Some(source),
                        &HSTRING::from(crate::ui_text::format(
                            "action-new-pin-failed",
                            &[("error", error.to_string().into())],
                        )),
                        w!("Otter"),
                        MB_OK | MB_ICONWARNING,
                    );
                }
            }
        }
        Ok(())
    }

    unsafe fn add_window(&mut self, visible: bool) -> Result<()> {
        self.add_state(State::new(None, false)?, visible)
    }
    unsafe fn add_state(&mut self, state: State, visible: bool) -> Result<()> {
        if self.documents.len() >= crate::workspace::MAX_UNITS {
            return Err(Error::new(
                E_OUTOFMEMORY,
                "Native document window budget exhausted",
            ));
        }
        self.documents
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate document owner"))?;
        let document = self.prepare_window(state, visible)?;
        self.workspace
            .add_units([document.state.unit_id.clone()])
            .map_err(|error| {
                Error::new(E_FAIL, format!("Cannot register workspace unit: {error:?}"))
            })?;
        let hwnd = document.window.0;
        let panel = document.panel.0;
        self.documents.push(document);
        self.sync_capture_windows();
        if visible {
            let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
            let _ = ShowWindow(panel, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }
    pub(super) unsafe fn run(mut self, startup: Option<super::SessionStartup>) -> Result<()> {
        let control = std::rc::Rc::clone(&self.session_end);
        super::session_end::with_owner(&control, || {
            // All main-loop errors go through shared cleanup first; neither ? nor a nonzero WM_QUIT may bypass the Live message pump.
            let result = self.run_loop(startup);
            self.collect_closed();
            self.shutdown();
            result?;
            self.error.take().map_or(Ok(()), Err)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_exit_wakeup_does_not_leave_a_latent_exit_request() {
        let state = State::new(None, false).unwrap();
        for _ in 0..2 {
            assert!(unsafe { request_exit(HWND(-2isize as *mut _), &state) }.is_err());
            assert!(!state.exit_requested.get());
        }
    }
    #[test]
    fn failed_notification_clears_request_and_modal_request_is_ignored() {
        let state = State::new(None, false).unwrap();
        state.file_dialog.set(true);
        unsafe {
            request(HWND::default(), &state).unwrap();
        }
        assert!(!state.new_window.get());
        state.file_dialog.set(false);
        // HWND -2 is not a valid target; NULL denotes a thread message and cannot serve as a failure fixture.
        assert!(unsafe { request(HWND(-2isize as *mut _), &state) }.is_err());
        assert!(!state.new_window.get());
    }
}
