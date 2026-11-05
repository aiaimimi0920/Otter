//! 连线选择的共享只读目录；只保留端口名和身份凭证，不持有像素或参数值。
use super::{
    configuration_stamp::Stamp, document_window::DocumentWindow, loom_result_state::OutputStamp,
    State,
};
use crate::{
    loom_capability::Capability,
    workspace::{Link, TopologyStamp, Workspace},
    workspace_id::UnitId,
};
use std::rc::Rc;
use windows::{
    core::Result,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) struct Endpoint {
    pub unit: UnitId,
    pub label: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub input_defaults: Vec<bool>,
    pub output_defaults: Vec<bool>,
    pub input_visibility: Rc<[bool]>,
    pub output_visibility: Rc<[bool]>,
    configuration: Option<(Stamp, OutputStamp)>,
    live: Option<std::sync::Arc<()>>,
}
pub(super) struct Catalog {
    pub endpoints: Vec<Endpoint>,
    topology: TopologyStamp,
    appearance: crate::workspace::ports::AppearanceStamp,
}
pub(super) struct Creation {
    pub catalog: Rc<Catalog>,
    pub link: Link,
}
impl Catalog {
    pub fn capture(
        documents: &[DocumentWindow],
        sessions: &[super::composition::live::session::Session],
        workspace: &Workspace,
    ) -> Rc<Self> {
        let mut endpoints: Vec<_> = documents
            .iter()
            .enumerate()
            .map(|(index, doc)| {
                let selection = doc.state.selected_art.borrow();
                let capability = selection.as_ref().and_then(|selected| {
                    selected
                        .catalog
                        .handshake
                        .arts()
                        .iter()
                        .filter_map(Capability::new)
                        .find(|cap| cap.id == selected.parameters.art_id())
                });
                let mut inputs = Vec::new();
                let mut outputs = Vec::new();
                let label = if let Some(capability) = &capability {
                    let parameters: Vec<_> = capability.parameters().collect();
                    for port in capability.inputs() {
                        if !parameters.iter().any(|p| p.id == port.name && p.secret) {
                            add(&mut inputs, port.name);
                        }
                    }
                    for parameter in parameters.iter().filter(|p| !p.secret) {
                        add(&mut inputs, parameter.id);
                    }
                    for port in capability.outputs() {
                        add(&mut outputs, port.name);
                    }
                    capability.label.to_owned()
                } else if let Some(id) = &doc.state.local_art_id {
                    inputs.extend(id.inputs().into_iter().map(|(name, _)| name));
                    outputs.extend(id.outputs());
                    id.to_string()
                } else {
                    add(&mut inputs, "image");
                    add(&mut outputs, "output");
                    "钉图".into()
                };
                let results = doc.state.execution_results.borrow();
                for port in results.ports() {
                    add(&mut outputs, &port.name);
                }
                let input_defaults: Vec<_> = inputs
                    .iter()
                    .map(|name| {
                        capability
                            .as_ref()
                            .map(|cap| cap.port_default_visible(name, true))
                            .unwrap_or_else(|| {
                                doc.state
                                    .local_art_id
                                    .as_ref()
                                    .is_none_or(|id| id.port_default_visible(name, true))
                            })
                    })
                    .collect();
                let output_defaults: Vec<_> = outputs
                    .iter()
                    .map(|name| {
                        capability
                            .as_ref()
                            .map(|cap| cap.port_default_visible(name, false))
                            .unwrap_or_else(|| {
                                doc.state
                                    .local_art_id
                                    .as_ref()
                                    .is_none_or(|id| id.port_default_visible(name, false))
                            })
                    })
                    .collect();
                let visibility = workspace.port_visibility(&doc.state.unit_id);
                let input_visibility = inputs
                    .iter()
                    .zip(&input_defaults)
                    .map(|(name, default)| visibility.get(name).unwrap_or(*default))
                    .collect();
                let output_visibility = outputs
                    .iter()
                    .zip(&output_defaults)
                    .map(|(name, default)| visibility.get(name).unwrap_or(*default))
                    .collect();
                Endpoint {
                    unit: doc.state.unit_id.clone(),
                    label: format!(
                        "{} · {}",
                        index + 1,
                        label
                            .chars()
                            .filter(|c| !c.is_control())
                            .take(128)
                            .collect::<String>()
                    ),
                    inputs,
                    outputs,
                    input_defaults,
                    output_defaults,
                    input_visibility,
                    output_visibility,
                    configuration: Some((Stamp::capture(&doc.state), results.output_stamp())),
                    live: None,
                }
            })
            .collect();
        for session in sessions
            .iter()
            .filter(|session| session.is_alive() && session.belongs_to(workspace))
        {
            endpoints.push(Endpoint {
                unit: session.unit_id().clone(),
                label: format!("{} · Live", endpoints.len() + 1),
                inputs: Vec::new(),
                outputs: vec!["output".into()],
                input_defaults: Vec::new(),
                output_defaults: vec![true],
                input_visibility: Rc::from([]),
                output_visibility: Rc::from([workspace
                    .port_visibility(session.unit_id())
                    .get("output")
                    .unwrap_or(true)]),
                configuration: None,
                live: Some(session.input_identity()),
            });
        }
        Rc::new(Self {
            endpoints,
            topology: workspace.topology_stamp(),
            appearance: workspace.appearance_stamp(),
        })
    }
    pub fn matches(
        &self,
        documents: &[DocumentWindow],
        sessions: &[super::composition::live::session::Session],
        workspace: &Workspace,
    ) -> bool {
        self.topology.matches(workspace)
            && self.appearance.matches(workspace)
            && self.endpoints.len()
                == documents.len()
                    + sessions
                        .iter()
                        .filter(|s| s.is_alive() && s.belongs_to(workspace))
                        .count()
            && self.endpoints.iter().zip(documents).all(|(endpoint, doc)| {
                endpoint.unit == doc.state.unit_id
                    && endpoint
                        .configuration
                        .as_ref()
                        .is_some_and(|(configuration, output)| {
                            configuration.matches(&doc.state)
                                && output.matches(&doc.state.execution_results.borrow())
                        })
            })
            && self.endpoints.iter().skip(documents.len()).all(|endpoint| {
                endpoint.live.as_ref().is_some_and(|identity| {
                    sessions.iter().any(|s| {
                        s.unit_id() == &endpoint.unit
                            && s.is_alive()
                            && s.belongs_to(workspace)
                            && std::sync::Arc::ptr_eq(identity, &s.input_identity())
                    })
                })
            })
    }
    pub fn permits(&self, link: &Link) -> bool {
        link.from_unit != link.to_unit
            && self
                .endpoints
                .iter()
                .any(|e| e.unit == link.from_unit && e.outputs.contains(&link.from_port))
            && self
                .endpoints
                .iter()
                .any(|e| e.unit == link.to_unit && e.inputs.contains(&link.to_port))
    }
    pub unsafe fn queue(
        self: &Rc<Self>,
        hwnd: HWND,
        state: &State,
        source: usize,
        output: usize,
        input: usize,
    ) -> Result<bool> {
        if !IsWindow(Some(hwnd)).as_bool()
            || state.file_dialog.get()
            || state.group_locked.get()
            || state.link_request.borrow().is_some()
            || state.link_creation.borrow().is_some()
            || !state
                .link_catalog
                .borrow()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, self))
        {
            return Ok(false);
        }
        let Some(target) = self.endpoints.iter().position(|e| e.unit == state.unit_id) else {
            return Ok(false);
        };
        let Some(link) = self.make_link(source, target, output, input)? else {
            return Ok(false);
        };
        *state.link_creation.borrow_mut() = Some(Creation {
            catalog: self.clone(),
            link,
        });
        if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
            state.link_creation.borrow_mut().take();
            return Err(error);
        }
        Ok(true)
    }
    pub fn make_link(
        &self,
        source: usize,
        target: usize,
        output: usize,
        input: usize,
    ) -> Result<Option<Link>> {
        let (Some(source), Some(target)) = (self.endpoints.get(source), self.endpoints.get(target))
        else {
            return Ok(None);
        };
        let (Some(from_port), Some(to_port)) =
            (source.outputs.get(output), target.inputs.get(input))
        else {
            return Ok(None);
        };
        let link = Link {
            id: super::document_state::new_unit_id()?,
            from_unit: source.unit.clone(),
            from_port: from_port.clone(),
            to_unit: target.unit.clone(),
            to_port: to_port.clone(),
        };
        if !self.permits(&link) {
            return Ok(None);
        }
        Ok(Some(link))
    }
}
fn add(ports: &mut Vec<String>, name: &str) {
    // 声明输入与公共参数各最多 128 项；去重后至多 256，不构造源/目标笛卡尔积。
    if ports.len() < 256 && !ports.iter().any(|port| port == name) {
        ports.push(name.into());
    }
}
