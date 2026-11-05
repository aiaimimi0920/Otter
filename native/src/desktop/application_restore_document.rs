//! 恢复事务的单窗口凭证：同时用于原工作区和隐藏候选，不持有像素副本。
use super::{idle, *};
use crate::desktop::{configuration_stamp, display_snapshot, loom_result_state::OutputStamp};

#[derive(PartialEq)]
struct Presentation {
    viewport: crate::viewport::Viewport,
    pinned: bool,
    window_opacity: f32,
    normal_opacity: f32,
    mini_opacity: f32,
    mini: Option<(crate::session_minified::Views, crate::crop_geometry::Size)>,
    restore: Option<crate::minified_transition::Transition>,
    locked: bool,
    hidden: bool,
    global_capture: bool,
    global_long_capture: bool,
    global_live_capture: bool,
    art: bool,
}

impl Presentation {
    fn capture(state: &crate::desktop::State) -> Self {
        let canvas = state.canvas.borrow();
        Self {
            viewport: canvas.viewport,
            pinned: canvas.pinned,
            window_opacity: canvas.window_opacity,
            normal_opacity: canvas.normal_opacity,
            mini_opacity: canvas.minified_opacity,
            mini: canvas.minified.as_ref().map(|mini| mini.geometry()),
            restore: canvas.minified_restore,
            locked: state.group_locked.get(),
            hidden: state.group_hidden.get(),
            global_capture: state.global_capture.get(),
            global_long_capture: state.global_long_capture.get(),
            global_live_capture: state.global_live_capture.get(),
            art: state.is_art(),
        }
    }
}
pub(in crate::desktop) struct Document {
    unit: crate::workspace_id::UnitId,
    window: HWND,
    rect: RECT,
    dpi: u32,
    display: display_snapshot::Stamp,
    configuration: configuration_stamp::Stamp,
    projection_edit: Option<crate::desktop::projection_edit_workspace::Stamp>,
    output: OutputStamp,
    presentation: Presentation,
}
impl Document {
    pub(in crate::desktop) unsafe fn capture(doc: &DocumentWindow) -> Result<Self> {
        if !doc.is_alive() || !idle::state(&doc.state) {
            return Err(Error::new(E_ABORT, "Restoration window is closed or busy"));
        }
        let mut rect = RECT::default();
        GetWindowRect(doc.window.0, &mut rect)?;
        let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(doc.window.0);
        if dpi == 0 {
            return Err(Error::new(E_ABORT, "Window DPI is unavailable"));
        }
        Ok(Self {
            unit: doc.state.unit_id.clone(),
            window: doc.window.0,
            rect,
            dpi,
            display: display_snapshot::Stamp::capture(
                &doc.state.canvas.borrow(),
                doc.state.resolved_display.borrow().as_ref(),
            ),
            configuration: configuration_stamp::Stamp::capture(&doc.state),
            projection_edit: crate::desktop::projection_edit_workspace::stamp(
                doc.state.projection_edit.borrow().as_ref(),
            ),
            output: doc.state.execution_results.borrow().output_stamp(),
            presentation: Presentation::capture(&doc.state),
        })
    }
    pub(in crate::desktop) unsafe fn matches(&self, doc: &DocumentWindow) -> bool {
        self.mismatch(doc).is_none()
    }
    /// 返回首个失配类别，诊断复用同一围栏，不另维护一套较宽松的条件。
    pub(in crate::desktop) unsafe fn mismatch(&self, doc: &DocumentWindow) -> Option<&'static str> {
        macro_rules! require {
            ($condition:expr, $reason:literal) => {
                if !$condition {
                    return Some($reason);
                }
            };
        }
        let mut rect = RECT::default();
        require!(self.unit == doc.state.unit_id, "unit");
        require!(self.window == doc.window.0, "window");
        require!(doc.is_alive(), "lifetime");
        require!(idle::state(&doc.state), "busy");
        require!(
            GetWindowRect(doc.window.0, &mut rect).is_ok(),
            "window-rect-query"
        );
        require!(rect == self.rect, "window-rect");
        require!(
            windows::Win32::UI::HiDpi::GetDpiForWindow(doc.window.0) == self.dpi,
            "dpi"
        );
        require!(
            self.display.matches(
                &doc.state.canvas.borrow(),
                doc.state.resolved_display.borrow().as_ref(),
            ),
            "display"
        );
        require!(self.configuration.matches(&doc.state), "configuration");
        require!(
            self.projection_edit
                == crate::desktop::projection_edit_workspace::stamp(
                    doc.state.projection_edit.borrow().as_ref()
                ),
            "projection-edit"
        );
        require!(
            self.output.matches(&doc.state.execution_results.borrow()),
            "output"
        );
        let presentation = Presentation::capture(&doc.state);
        require!(
            self.presentation.viewport == presentation.viewport,
            "viewport"
        );
        require!(self.presentation == presentation, "presentation");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::{Apartment, State};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;

    struct Class(HINSTANCE);
    impl Drop for Class {
        fn drop(&mut self) {
            unsafe {
                let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
            }
        }
    }

    #[test]
    fn restoration_document_diagnostics_preserve_first_mismatch_and_reject_all_changed_guards() {
        let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _com = Apartment::new().unwrap();
        unsafe {
            let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
            assert_ne!(
                RegisterClassW(&WNDCLASSW {
                    hInstance: instance,
                    lpszClassName: w!("Otter.Native.Workspace"),
                    lpfnWndProc: Some(crate::desktop::procedure::window_proc),
                    ..Default::default()
                }),
                0
            );
            let _class = Class(instance); // 成功注册才取得所有权；在 doc 销毁后注销。
            let doc =
                DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
            let check = |guard: &Document, expected: Option<&str>| {
                assert_eq!(guard.mismatch(&doc), expected);
                assert_eq!(guard.matches(&doc), expected.is_none());
            };
            let mut guard = Document::capture(&doc).unwrap();
            check(&guard, None);
            guard.rect.left -= 1;
            guard.dpi = guard.dpi.saturating_add(1);
            guard.presentation.viewport.origin.x += 1.0;
            check(&guard, Some("window-rect"));
            doc.state.new_window.set(true);
            check(&guard, Some("busy"));
            assert!(doc.state.new_window.get()); // 诊断不消费待处理请求。
            doc.state.new_window.set(false);
            guard.rect.left += 1;
            check(&guard, Some("dpi"));
            guard.dpi -= 1;
            check(&guard, Some("viewport"));
            guard.presentation.viewport.origin.x -= 1.0;
            check(&guard, None);
            doc.state.canvas.borrow_mut().normal_opacity = 0.5;
            check(&guard, Some("display"));
            doc.state.canvas.borrow_mut().normal_opacity = guard.presentation.normal_opacity;
            check(&guard, None);
            doc.state.group_hidden.set(!guard.presentation.hidden);
            check(&guard, Some("presentation"));
            doc.state.group_hidden.set(guard.presentation.hidden);
            check(&guard, None);
        }
    }
}
