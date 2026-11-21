//! 外层消息结算同步扩展视图；目录/源/附件未变时只比较凭证，不解析 JSON 或重建文字布局。
use super::*;
impl Application {
    pub(super) unsafe fn sync_extension_views(&mut self) {
        let catalog_changed = self.extensions.process();
        self.sync_extension_shortcuts();
        self.extension_commands
            .process(&self.documents, &self.naming_document);
        for document in &self.documents {
            if !document.is_alive() {
                continue;
            }
            if let Err(error) = super::super::extension_notices::sync(&document.state) {
                eprintln!("native-extension-notice: {error}");
            }
            let changed =
                match super::super::extension_overlay::sync(document.window.0, &document.state) {
                    Ok(changed) => changed,
                    Err(error) => {
                        eprintln!("native-extension-view: {error}");
                        true
                    }
                };
            if changed || catalog_changed {
                crate::desktop::tool_panel::refresh(document.window.0);
            }
        }
    }
}
