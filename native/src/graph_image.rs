//! 图像来源解析只借用引用；不解码、不复制像素，Art 预览与正式输出保持分离。
use crate::{
    workspace::{Link, MAX_LINKS, MAX_UNITS},
    workspace_id::UnitId,
};
pub const MAX_PORTS: usize = 128;
#[path = "graph_canvas_display.rs"]
mod canvas_display;
#[path = "graph_dispatch.rs"]
mod dispatch;
#[path = "graph_execution_images.rs"]
mod execution;
#[path = "graph_notifications.rs"]
mod notifications;
#[path = "graph_output.rs"]
mod output_value;
#[path = "graph_effective_params.rs"]
mod params;
pub use dispatch::{DispatchOptions, Preparation, Prepared, SkipReason};
pub(crate) use output_value::validate as validate_outputs;
pub use output_value::Output;

#[derive(Clone, Copy)]
pub struct Port<'a> {
    pub name: &'a str,
    pub kind: &'a str,
}
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Sticker,
    Art,
    Other,
}
pub struct Node<'a> {
    pub id: &'a UnitId,
    pub kind: Kind,
    pub inputs: &'a [Port<'a>],
    pub outputs: &'a [Port<'a>],
    /// Some(empty) 明确表示能力声明没有输入，不能退回对象自带端口。
    pub capability_inputs: Option<&'a [Port<'a>]>,
    pub capability_outputs: &'a [Port<'a>],
    pub values: &'a [(&'a str, Output<'a>)],
    pub source: Option<&'a str>,
    pub preview: Option<&'a str>,
    pub manual_image: Option<&'a str>,
    pub input_disabled: bool,
    pub accept_upstream: bool,
    pub locally_edited: bool,
    pub pending: bool,
}
impl Node<'_> {
    fn relays(&self) -> bool {
        !self.input_disabled && self.accept_upstream && !self.locally_edited
    }
    fn image_input(&self, name: &str) -> bool {
        let ports = if self.kind == Kind::Art {
            self.capability_inputs.unwrap_or(self.inputs)
        } else {
            self.inputs
        };
        let has_image = ports
            .iter()
            .any(|port| !port.name.is_empty() && image_like(*port));
        if self.kind != Kind::Art && !has_image {
            return name == "image";
        }
        ports
            .iter()
            .any(|port| !port.name.is_empty() && port.name == name && image_like(*port))
    }
    fn image_output(&self, name: &str) -> bool {
        ["output", "output_image", "image", "result", "preview"]
            .iter()
            .any(|alias| name.eq_ignore_ascii_case(alias))
            || (self.kind == Kind::Art
                && self
                    .capability_outputs
                    .iter()
                    .any(|port| port.name == name && image_like(*port)))
            || self
                .outputs
                .iter()
                .any(|port| port.name == name && image_like(*port))
    }
}
fn image_like(port: Port<'_>) -> bool {
    let suffix = |value: &[u8]| {
        port.name
            .as_bytes()
            .get(port.name.len().saturating_sub(value.len())..)
            .is_some_and(|tail| tail.eq_ignore_ascii_case(value))
    };
    port.kind
        .as_bytes()
        .windows(5)
        .any(|part| part.eq_ignore_ascii_case(b"image"))
        || ["image", "input_image", "input"]
            .iter()
            .any(|name| port.name.eq_ignore_ascii_case(name))
        || suffix(b"_image")
        || suffix(b"_file")
}

pub struct Graph<'a> {
    nodes: &'a [Node<'a>],
    links: &'a [Link],
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Capacity,
    DuplicateId,
    Allocation,
    InvalidOutput,
}
impl<'a> Graph<'a> {
    pub fn new(nodes: &'a [Node<'a>], links: &'a [Link]) -> Result<Self, Error> {
        if nodes.len() > MAX_UNITS || links.len() > MAX_LINKS {
            return Err(Error::Capacity);
        }
        // 借用视图也校验元数据工作量，不能让插件提供无界端口数组拖慢 UI 解析。
        for node in nodes {
            for ports in [
                node.inputs,
                node.outputs,
                node.capability_inputs.unwrap_or(&[]),
                node.capability_outputs,
            ] {
                if ports.len() > MAX_PORTS
                    || ports
                        .iter()
                        .any(|port| port.name.len() > 256 || port.kind.len() > 256)
                {
                    return Err(Error::Capacity);
                }
            }
            if node.values.len() > MAX_PORTS || node.values.iter().any(|(name, _)| name.len() > 256)
            {
                return Err(Error::Capacity);
            }
            output_value::validate(node.values)?;
        }
        if nodes
            .iter()
            .enumerate()
            .any(|(index, node)| nodes[..index].iter().any(|other| other.id == node.id))
        {
            return Err(Error::DuplicateId);
        }
        Ok(Self { nodes, links })
    }
    fn index(&self, id: &UnitId) -> Option<usize> {
        self.nodes.iter().position(|node| node.id == id)
    }
    fn input(&self, index: usize) -> Option<&Link> {
        let node = &self.nodes[index];
        self.links
            .iter()
            .find(|link| &link.to_unit == node.id && node.image_input(&link.to_port))
    }
    pub fn image(&self, id: &UnitId) -> Option<&'a str> {
        self.resolve(self.index(id)?, &mut [false; MAX_UNITS])
    }
    pub fn output(&self, id: &UnitId, port: &str) -> Option<Output<'a>> {
        self.output_inner(self.index(id)?, port, &mut [false; MAX_UNITS])
    }
    pub fn connected_image(&self, id: &UnitId, port: &str) -> Option<&'a str> {
        let link = self
            .links
            .iter()
            .find(|link| &link.to_unit == id && link.to_port == port)?;
        self.connected(link, &mut [false; MAX_UNITS])
    }
    fn output_inner(
        &self,
        index: usize,
        port: &str,
        visited: &mut [bool; MAX_UNITS],
    ) -> Option<Output<'a>> {
        let node = &self.nodes[index];
        if let Some((_, value)) = node.values.iter().find(|(name, _)| *name == port) {
            return Some(*value);
        }
        if port != "output" {
            if let Some((_, value)) = node.values.iter().find(|(name, _)| *name == "output") {
                return Some(*value);
            }
        }
        if node.kind == Kind::Art || !node.image_output(port) {
            return None;
        }
        self.resolve(index, visited).map(Output::Image)
    }
    fn connected(&self, link: &Link, visited: &mut [bool; MAX_UNITS]) -> Option<&'a str> {
        let port = if link.from_port.is_empty() {
            "output"
        } else {
            &link.from_port
        };
        match self
            .output_inner(self.index(&link.from_unit)?, port, visited)?
            .direct()
        {
            Output::Image(value) if !value.is_empty() => Some(value),
            _ => None,
        }
    }
    fn resolve(&self, index: usize, visited: &mut [bool; MAX_UNITS]) -> Option<&'a str> {
        if visited[index] {
            return None;
        }
        visited[index] = true;
        let node = &self.nodes[index];
        if node.kind == Kind::Sticker {
            if node.relays() {
                if let Some(image) = self
                    .input(index)
                    .and_then(|link| self.connected(link, visited))
                {
                    return Some(image);
                }
                if let Some(image) = node.manual_image.filter(|image| image.starts_with("data:")) {
                    return Some(image);
                }
            }
            return node
                .preview
                .filter(|value| !value.is_empty())
                .or(node.source);
        }
        if let Some(preview) = node.preview.filter(|value| !value.is_empty()) {
            return Some(preview);
        }
        if let Some(image) = self
            .input(index)
            .and_then(|link| self.index(&link.from_unit))
            .and_then(|source| self.resolve(source, visited))
        {
            return Some(image);
        }
        node.source
    }
    pub fn formal_pending(&self, id: &UnitId) -> bool {
        let Some(mut index) = self.index(id) else {
            return false;
        };
        let mut visited = [false; MAX_UNITS];
        loop {
            if visited[index] {
                return false;
            }
            visited[index] = true;
            let node = &self.nodes[index];
            if node.kind == Kind::Art {
                return node.pending;
            }
            if !node.relays() {
                return false;
            }
            let Some(next) = self
                .input(index)
                .and_then(|link| self.index(&link.from_unit))
            else {
                return false;
            };
            index = next;
        }
    }
}
#[cfg(test)]
#[path = "graph_json_tests.rs"]
mod json_tests;
#[cfg(test)]
#[path = "graph_image_tests.rs"]
mod tests;
