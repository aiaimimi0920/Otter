//! Reap remote sessions at application message-settlement points; window callbacks neither show terminal dialogs nor wait on background threads.
use super::*;
#[path = "application_surface_javascript.rs"]
mod javascript;
#[path = "application_surface_resources.rs"]
mod resources;
impl Application {
    pub(super) unsafe fn poll_remote_live(&mut self, visible: bool) -> Result<()> {
        self.poll_remote_live_with(
            visible,
            crate::device_authorization_config::Config::from_env,
        )
    }
    unsafe fn poll_remote_live_with(
        &mut self,
        visible: bool,
        mut config: impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
    ) -> Result<()> {
        let mut warnings = Vec::new();
        let result = self.settle_remote_live_with(visible, &mut config, &mut warnings);
        if warnings.is_empty() {
            return result;
        }
        // One-shot results have left the queue; finish this settlement before reporting together, without entering a modal message pump during settlement.
        if !visible {
            let code = result.err().map_or(E_FAIL, |error| {
                let code = error.code();
                warnings.push(error.to_string());
                code
            });
            return Err(Error::new(code, warnings.join("\n\n")));
        }
        MessageBoxW(
            None,
            &HSTRING::from(warnings.join("\n\n")),
            w!("Otter"),
            MB_OK | MB_ICONWARNING,
        );
        result
    }

    fn settle_remote_live_with(
        &mut self,
        visible: bool,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
        warnings: &mut Vec<String>,
    ) -> Result<()> {
        self.poll_live_sources_with(config);
        self.remote_sessions
            .expire_surface_resources(crate::desktop::surface_windows::now_ms()?);
        let windows = &self.surface_windows;
        self.remote_sessions
            .cancel_surface_resources(|binding, revision| {
                !windows.retains_resource(binding, revision)
            });
        for (origin, error) in self.remote_sessions.poll_surfaces() {
            warnings.push(crate::ui_text::format(
                "app-remote-surface-failed",
                &[
                    ("origin", origin.into()),
                    ("error", format!("{error:?}").into()),
                ],
            ));
        }
        self.poll_surface_recovery_ui();
        if let Some(error) = self.remote_sessions.poll_catalog() {
            let mut view = self.authorization.borrow_mut();
            view.catalog_busy = false;
            view.catalog = None;
            view.status = crate::ui_text::format(
                "app-catalog-query-failed",
                &[("error", format!("{error:?}").into())],
            );
            warnings.push(crate::ui_text::format(
                "app-remote-catalog-failed",
                &[("error", format!("{error:?}").into())],
            ));
        }
        if let Some(catalog) = self.remote_sessions.take_catalog() {
            let mut view = self.authorization.borrow_mut();
            if !view.closed && view.origin.as_deref() == Some(&catalog.origin) {
                view.catalog_busy = false;
                view.status = crate::ui_text::format(
                    "app-remote-catalog-count",
                    &[("count", catalog.entries.len().into())],
                );
                view.catalog = Some(catalog);
            }
        }
        if !self.remote_sessions.discovering() {
            let mut view = self.authorization.borrow_mut();
            if view.catalog_busy {
                view.catalog_busy = false;
                view.status = crate::ui_text::text("app-remote-catalog-cancelled").into();
            }
        }
        self.settle_live_viewers()?;
        for event in self.remote_sessions.poll() {
            self.authorization.borrow_mut().live.status =
                crate::ui_text::text("app-remote-viewing-ended").into();
            if let Some(error) = event.outcome.error_message() {
                warnings.push(crate::ui_text::format(
                    "app-remote-viewing-stopped",
                    &[
                        ("origin", event.origin.into()),
                        ("error", error.to_string().into()),
                    ],
                ));
            }
        }
        // Reap catalog/viewer/image completion slots before scheduling JS; settle lifecycle last so HWNDs closed by background failure
        // or synchronous start failure become inactive this turn, without losing the final wakeup source.
        self.settle_surface_images(crate::desktop::surface_windows::now_ms()?);
        self.poll_surface_javascript_with(visible, config)?;
        if self.activate_surface_windows_with(visible, config)? {
            let text = crate::ui_text::text("app-surface-recovery-full");
            if visible {
                warnings.push(text.into());
            } else {
                self.authorization.borrow_mut().recovery_status = text.into();
            }
        }
        self.settle_live_viewers()?;
        let windows = &self.surface_windows;
        self.remote_sessions
            .cancel_surface_resources(|binding, revision| {
                !windows.retains_resource(binding, revision)
            });
        {
            let mut view = self.authorization.borrow_mut();
            let records = self.surface_windows.retired_records();
            let already_listed = view.recent_surface_recovery.len();
            // The staging batch is append-only; an existing menu command keeps its identity.
            view.recent_surface_recovery.extend(
                records
                    .iter()
                    .skip(already_listed)
                    .cloned()
                    .map(std::sync::Arc::new),
            );
            view.mount_busy = self.remote_sessions.attaching() || self.local_arts.busy();
            if view.restoring && !view.mount_busy {
                view.restoring = false;
                view.recovery_status.clear();
            }
            if !view.mount_busy && !view.busy {
                if let Some(status) = self.surface_windows.status() {
                    view.status = status;
                }
            }
        }
        self.schedule_surface_images_with(config)?;
        Ok(())
    }
}
