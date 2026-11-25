//! Settle real Application JS mounts, sharing original Grant/task capacity with images; do not prepare Client on the UI thread.
use super::*;
impl Application {
    pub(super) fn poll_surface_javascript_with(
        &mut self,
        visible: bool,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
    ) -> Result<()> {
        let now = crate::desktop::surface_windows::now_ms()?;
        // Tighten the same application input policy before delivering replies; old requests after modal entry/revocation cannot steal focus during refresh.
        let origin = self.surface_input_origin(visible);
        for window in self.surface_windows.iter_mut() {
            if origin.as_deref() != Some(window.binding().origin()) {
                window.disable_local_input()?;
            }
        }
        self.surface_windows.poll(now)?;
        let mut running = self
            .surface_windows
            .iter_mut()
            .filter(|w| !w.javascript_finished())
            .count();
        for window in self.surface_windows.iter_mut() {
            if !window.javascript_needs_start() {
                continue;
            }
            if running >= crate::surface_javascript::MAX_SESSIONS {
                break;
            }
            let origin = window.binding().origin();
            let lease = match self.authorization_owner.lease(origin, now) {
                Ok(lease) => lease,
                Err(crate::device_authorization_owner::Error::Capacity) => continue,
                Err(error) => {
                    window.fail_javascript(crate::ui_text::format(
                        "app-js-authorization-unavailable",
                        &[("error", format!("{error:?}").into())],
                    ))?;
                    continue;
                }
            };
            let config = match config(origin) {
                Ok(config) => config,
                Err(error) => {
                    window.fail_javascript(crate::ui_text::format(
                        "app-js-configuration-invalid",
                        &[("error", format!("{error:?}").into())],
                    ))?;
                    continue;
                }
            };
            if let Err(error) = window.start_javascript(config, lease) {
                // Before startup, capacity contention may retry after existing tasks finish; once Runtime exists, never replay mounting.
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    continue;
                }
                window.fail_javascript(crate::ui_text::format(
                    "app-js-mount-not-started",
                    &[("error", error.to_string().into())],
                ))?;
            } else {
                running += 1;
            }
        }
        Ok(())
    }
}
