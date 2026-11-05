//! 原子准备多端口输出，管理本地像素/原始 PNG；远端描述符的租约仍归调用方释放。
use super::{image::Image, loom_output_inline, loom_output_shared};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
#[path = "loom_output_candidates.rs"]
pub(crate) mod candidates;
#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidOutputs,
    InvalidCandidates,
    Unsupported,
    Decode,
    Cancelled,
    Budget,
    Allocation,
    Consumed,
}
pub enum Output {
    Image(Image),
    PngImage(super::loom_image_value::ImageValue),
    Value(Value),
}
impl Output {
    pub fn image(&self) -> Option<&Image> {
        match self {
            Self::Image(image) => Some(image),
            Self::PngImage(value) => Some(value.image()),
            Self::Value(_) => None,
        }
    }
    pub(crate) fn image_value(&self) -> Option<super::loom_image_value::ImageValue> {
        match self {
            Self::Image(image) => Some(image.clone().into()),
            Self::PngImage(value) => Some(value.clone()),
            Self::Value(_) => None,
        }
    }
    pub(crate) fn from_image(value: super::loom_image_value::ImageValue) -> Self {
        if value.encoded_png().is_some() {
            Self::PngImage(value)
        } else {
            Self::Image(value.into_image())
        }
    }
}
pub struct Port {
    pub name: String,
    pub value: Output,
}
pub struct Batch {
    ports: Vec<Port>,
    primary: usize,
    pub(crate) candidates: Option<std::sync::Arc<candidates::Set>>,
}
impl Batch {
    pub(crate) fn local_image(value: super::loom_image_value::ImageValue) -> Result<Self, Error> {
        let mut ports = Vec::new();
        ports.try_reserve_exact(1).map_err(|_| Error::Allocation)?;
        ports.push(Port {
            name: "output_image".into(),
            value: Output::from_image(value),
        });
        Ok(Self {
            ports,
            primary: 0,
            candidates: None,
        })
    }
    pub fn ports(&self) -> &[Port] {
        &self.ports
    }
    pub fn primary(&self) -> &Port {
        &self.ports[self.primary]
    }
    /// 将端口数据直接转交节点状态，不克隆 JSON 或像素；主端口索引保持不变。
    pub fn into_ports(self) -> (Vec<Port>, usize) {
        (self.ports, self.primary)
    }
}

/// 不消费原始 JSON；无论成功失败，调用方都仍持有远端资源句柄和 candidates 元数据。
pub fn decode(
    outputs: &Value,
    candidates: Option<&Value>,
    cancel: &AtomicBool,
) -> Result<Batch, Error> {
    let map = outputs
        .as_object()
        .filter(|map| !map.is_empty() && map.len() <= 128)
        .ok_or(Error::InvalidOutputs)?;
    if candidates
        .and_then(|value| value.get("items"))
        .and_then(Value::as_array)
        .is_some_and(|items| items.len() > 64)
    {
        return Err(Error::InvalidCandidates);
    }
    let mut ports = Vec::new();
    ports
        .try_reserve_exact(map.len())
        .map_err(|_| Error::Allocation)?;
    let mut bytes = 0usize;
    for (name, value) in map {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
            return Err(Error::InvalidOutputs);
        }
        let decoded = match value["kind"].as_str() {
            Some("value") => {
                Output::Value(value.get("value").ok_or(Error::InvalidOutputs)?.clone())
            }
            Some("inline_resource") => Output::Image(
                loom_output_inline::decode(value, cancel).map_err(|_| decode_error(cancel))?,
            ),
            Some("shared_memory") => {
                let descriptor =
                    loom_output_shared::Descriptor::parse(value).map_err(|_| Error::Decode)?;
                Output::Image(
                    loom_output_shared::read(&descriptor, cancel)
                        .map_err(|_| decode_error(cancel))?,
                )
            }
            _ => return Err(Error::Unsupported),
        };
        if let Output::Image(image) = &decoded {
            bytes = bytes.checked_add(image.pixels.len()).ok_or(Error::Budget)?;
            if bytes > 256 * 1024 * 1024 {
                return Err(Error::Budget);
            }
        }
        ports.push(Port {
            name: name.clone(),
            value: decoded,
        });
    }
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    let primary = ["output_image", "output", "image"]
        .iter()
        .find_map(|name| ports.iter().position(|port| port.name == *name))
        .unwrap_or(0);
    let candidates = candidates::Set::prepare(candidates, cancel)?;
    Ok(Batch {
        ports,
        primary,
        candidates,
    })
}
fn decode_error(cancel: &AtomicBool) -> Error {
    if cancel.load(Ordering::Acquire) {
        Error::Cancelled
    } else {
        Error::Decode
    }
}

#[cfg(test)]
#[path = "loom_output_batch_tests.rs"]
mod tests;
