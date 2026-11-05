//! 参数窗是有界文档 owner；隐藏/托盘/关闭销毁草稿，动作在既有 UI 调度段串行提交。
use super::*;
use crate::desktop::document_window::parameter_panel::{self, Panel};
const MAX_OPEN: usize = 4;
#[cfg(test)]
#[path = "application_parameter_panel_tests.rs"]
mod tests;
impl Application {
    pub(super) unsafe fn process_parameter_panels(&mut self, visible: bool) {
        let mut open = self
            .documents
            .iter()
            .filter(|doc| doc.parameters.is_some())
            .count();
        let mut changed = false;
        for doc in &mut self.documents {
            let close = doc.state.parameters_ui.close.replace(false)
                || doc.state.group_hidden.get()
                || !doc.is_alive()
                || doc.parameters.as_ref().is_some_and(|panel| panel.retired());
            let toggle = doc.state.parameters_ui.toggle.replace(false);
            if close || toggle && doc.parameters.is_some() {
                if doc.parameters.take().is_some() {
                    open -= 1;
                    changed = true;
                }
                continue;
            }
            if toggle && parameter_panel::available(&doc.state) {
                let result = if open >= MAX_OPEN {
                    Err(Error::new(E_OUTOFMEMORY, "最多同时打开4个参数面板"))
                } else {
                    Panel::new(doc.window.0, &doc.state)
                };
                match result {
                    Ok(panel) => {
                        if visible {
                            panel.show();
                        }
                        doc.parameters = Some(panel);
                        open += 1;
                        changed = true;
                    }
                    Err(error) => {
                        eprintln!("native-parameter-panel: {error}");
                        if visible && doc.is_alive() {
                            MessageBoxW(
                                Some(doc.window.0),
                                &HSTRING::from(format!("无法打开参数面板。\n{error}")),
                                w!("Otter"),
                                MB_OK | MB_ICONWARNING,
                            );
                        }
                    }
                }
            }
            if let Some(panel) = &doc.parameters {
                if let Err(error) = panel.update(&doc.state) {
                    eprintln!("native-parameter-panel-update: {error}");
                    doc.parameters.take();
                    open -= 1;
                    changed = true;
                }
            }
        }
        if changed {
            self.sync_capture_windows();
        }
    }
}
