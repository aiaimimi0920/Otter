//! Documents own only Local Art identity/port hints, without fabricating selected_art or Backend execution configuration.
use crate::art_execution_grant::package::ports::Record;
use std::{cell::RefCell, ops::Deref, sync::Arc};
pub(in crate::desktop) struct Config {
    id: String,
    pub ports: RefCell<Option<Arc<Record>>>,
    // Retract only tentative edges created by this factory and untouched by users; restored graphs lack this authority.
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
