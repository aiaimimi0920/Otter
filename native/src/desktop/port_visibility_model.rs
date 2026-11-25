//! Dialog drafts hold only public names and display policies; same-named ports on both sides share overrides under the original portVisibility contract.
use crate::{desktop::link_catalog::Endpoint, workspace::ports::Visibility};
pub(super) struct Row {
    pub name: String,
    pub label: String,
    pub default_visible: bool,
}
pub(in crate::desktop::application::chrome::ports) struct Model {
    pub(super) rows: Vec<Row>,
    pub(super) visibility: Visibility,
}
impl Model {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn new(endpoint: &Endpoint, visibility: Visibility) -> Self {
        let mut rows = Vec::new();
        for (message, ports, defaults) in [
            (
                "port-input-label",
                &endpoint.inputs,
                &endpoint.input_defaults,
            ),
            (
                "port-output-label",
                &endpoint.outputs,
                &endpoint.output_defaults,
            ),
        ] {
            for (index, name) in ports.iter().enumerate() {
                rows.push(Row {
                    name: name.clone(),
                    label: crate::ui_text::format(
                        message,
                        &[(
                            "name",
                            name.chars()
                                .map(|c| if c.is_control() { ' ' } else { c })
                                .collect::<String>()
                                .into(),
                        )],
                    ),
                    default_visible: defaults.get(index).copied().unwrap_or(true),
                });
            }
        }
        Self { rows, visibility }
    }
    pub fn mode(&self, index: usize) -> Option<usize> {
        Some(match self.visibility.get(&self.rows.get(index)?.name) {
            None => 0,
            Some(true) => 1,
            Some(false) => 2,
        })
    }
    pub fn edit(&mut self, index: usize, mode: usize) -> Result<(), crate::workspace::Error> {
        let row = self
            .rows
            .get(index)
            .ok_or(crate::workspace::Error::InvalidPort)?;
        let value = match mode {
            0 => None,
            1 => Some(true),
            2 => Some(false),
            _ => return Err(crate::workspace::Error::InvalidPort),
        };
        self.visibility = self.visibility.edited(&row.name, value)?;
        Ok(())
    }
}
