//! 文档只拥有 Local Art 身份与端口提示，不伪造 selected_art 或 Backend 执行配置。
use crate::art_execution_grant::package::ports::Record;
use std::{cell::RefCell, ops::Deref, sync::Arc};
pub(in crate::desktop) struct Config {
    id: String,
    pub ports: RefCell<Option<Arc<Record>>>,
    // 只允许撤回本次工厂自建且未被用户改动的暂定边；恢复的图没有这个权限。
    pub pending_input: RefCell<Option<crate::workspace::Link>>,
}
impl From<String> for Config {
    fn from(id: String) -> Self {
        Self {
            id,
            ports: RefCell::new(None),
            pending_input: RefCell::new(None),
        }
    }
}
impl Deref for Config {
    type Target = str;
    fn deref(&self) -> &str {
        &self.id
    }
}
impl Config {
    pub fn port_default_visible(&self, name: &str, input: bool) -> bool {
        self.ports.borrow().as_ref().is_none_or(|record| {
            record
                .visibility
                .as_ref()
                .is_none_or(|visibility| visibility.visible(&record.ports, name, input))
        })
    }
    pub fn inputs(&self) -> Vec<(String, String)> {
        self.ports
            .borrow()
            .as_ref()
            .map(|r| {
                r.ports
                    .inputs
                    .iter()
                    .map(|p| (p.name.clone(), p.kind.clone()))
                    .collect()
            })
            .unwrap_or_else(|| vec![("input_image".into(), "image".into())])
    }
    pub fn outputs(&self) -> Vec<String> {
        self.ports
            .borrow()
            .as_ref()
            .map(|r| r.ports.outputs.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_else(|| vec!["output_image".into()])
    }
    pub fn primary_output(&self) -> Option<String> {
        match self.ports.borrow().as_ref() {
            Some(record) => record
                .ports
                .outputs
                .iter()
                .find(|port| port.kind.to_ascii_lowercase().contains("image"))
                .map(|port| port.name.clone()),
            None => Some("output_image".into()),
        }
    }
}
