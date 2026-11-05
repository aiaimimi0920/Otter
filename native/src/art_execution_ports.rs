//! 当前包的严格端口 DTO；旧 v2 reader 仅留在测试中验证签名域隔离。
use super::*;
use serde::{Deserialize, Serialize};
#[cfg(test)]
pub(crate) const PROTOCOL: &str = "loom.art.local.package.v2";
#[cfg(test)]
pub(crate) const DOMAIN: &[u8] = b"loom.art.local.package.v2\0";
#[path = "art_local_ports_record.rs"]
mod record;
pub(crate) use record::Record;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Port {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Ports {
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
}
fn text(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
impl Ports {
    pub(crate) fn primary_input(&self) -> Option<&str> {
        self.inputs
            .iter()
            .find(|p| {
                p.kind.to_ascii_lowercase().contains("image")
                    || ["input", "input_image", "image"]
                        .contains(&p.name.to_ascii_lowercase().as_str())
            })
            .or(self.inputs.first())
            .map(|p| p.name.as_str())
    }
    pub(crate) fn validate(&self) -> Result<(), Error> {
        for ports in [&self.inputs, &self.outputs] {
            if ports.len() > 32
                || ports.iter().enumerate().any(|(i, p)| {
                    !text(&p.name, 128)
                        || !text(&p.kind, 64)
                        || ports[..i].iter().any(|other| other.name == p.name)
                })
            {
                return Err(Error::Source);
            }
        }
        if serde_json::to_vec(self).map_err(|_| Error::Wire)?.len() > 8 * 1024 {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
#[cfg(test)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Package {
    pub protocol_version: String,
    pub package: super::Package,
    pub ports: Ports,
}
#[cfg(test)]
impl Package {
    pub(crate) fn verify(bytes: &[u8], issuer: Issuer<'_>) -> Result<Self, Error> {
        let payload = signed_payload(bytes, issuer, DOMAIN)?;
        let package: Self = serde_json::from_slice(&payload).map_err(|_| Error::Wire)?;
        if package.protocol_version != PROTOCOL {
            return Err(Error::Wire);
        }
        package.package.validate()?;
        package.ports.validate()?;
        Ok(package)
    }
}
