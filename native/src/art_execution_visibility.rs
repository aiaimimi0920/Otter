//! v3 将默认显隐和原端口一起签名；不改变严格 v2 reader、执行权限或包资源预算。
use super::*;
use serde::{Deserialize, Serialize};
pub(crate) const PROTOCOL: &str = "loom.art.local.package.v3";
pub(crate) const DOMAIN: &[u8] = b"loom.art.local.package.v3\0";
pub(crate) const PREPARE: &str = "loom.art.local.prepare-package.v3";
pub(crate) const ROUTE: &str = "/v3/art/local/prepare-package";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Visibility {
    pub inputs: Vec<bool>,
    pub outputs: Vec<bool>,
}
impl Visibility {
    pub(crate) fn validate(&self, ports: &ports::Ports) -> Result<(), Error> {
        ports.validate()?;
        if self.inputs.len() != ports.inputs.len() || self.outputs.len() != ports.outputs.len() {
            return Err(Error::Source);
        }
        if serde_json::to_vec(&(ports, self))
            .map_err(|_| Error::Wire)?
            .len()
            > 8 * 1024
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
    pub(crate) fn visible(&self, ports: &ports::Ports, name: &str, input: bool) -> bool {
        let (ports, values) = if input {
            (&ports.inputs, &self.inputs)
        } else {
            (&ports.outputs, &self.outputs)
        };
        ports
            .iter()
            .position(|port| port.name == name)
            .and_then(|i| values.get(i))
            .copied()
            .unwrap_or(true)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Package {
    pub protocol_version: String,
    pub package: super::Package,
    pub ports: ports::Ports,
    pub visibility: Visibility,
}
impl Package {
    pub(crate) fn verify(bytes: &[u8], issuer: Issuer<'_>) -> Result<Self, Error> {
        let payload = signed_payload(bytes, issuer, DOMAIN)?;
        let proof: Self = serde_json::from_slice(&payload).map_err(|_| Error::Wire)?;
        if proof.protocol_version != PROTOCOL {
            return Err(Error::Wire);
        }
        proof.package.validate()?;
        proof.visibility.validate(&proof.ports)?;
        Ok(proof)
    }
}
