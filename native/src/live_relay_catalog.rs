//! 已认证会话选择目录：全量验证、去歧义和有界输出，不包含凭据或观察业务快照。
use super::*;
use serde::Deserialize;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub identity: Identity,
    pub revision: u64,
    pub member: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    protocol_version: String,
    sessions: Vec<Box<serde_json::value::RawValue>>,
}
#[derive(Deserialize)]
struct Header {
    session: SessionHeader,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionHeader {
    session_id: String,
}
impl Control<'_> {
    /// 列表只是选择候选；加入时必须再以 identity 校验来源/epoch，不能把列表当加入授权。
    pub fn catalog(&self) -> Result<Vec<CatalogEntry>, Error> {
        let bytes = self.send("GET", "/v1/live/sessions", &[], Duration::from_secs(10))?;
        parse(&bytes, self.session.device_id())
    }
}
fn parse(bytes: &[u8], device: &str) -> Result<Vec<CatalogEntry>, Error> {
    let catalog: Catalog = protocol::parse(bytes).map_err(Error::Protocol)?;
    if catalog.protocol_version != protocol::PROTOCOL || catalog.sessions.len() > 512 {
        return Err(Error::Discovery);
    }
    let mut entries = Vec::with_capacity(catalog.sessions.len());
    let mut ids = std::collections::BTreeSet::new();
    for raw in catalog.sessions {
        let header: Header = serde_json::from_str(raw.get()).map_err(|_| Error::Discovery)?;
        path(&header.session.session_id, None)?;
        if !ids.insert(header.session.session_id.clone()) {
            return Err(Error::Discovery);
        }
        let snapshot = Snapshot::parse(raw.get().as_bytes(), &header.session.session_id)
            .map_err(Error::Protocol)?;
        entries.push(CatalogEntry {
            member: snapshot.renewal(snapshot.identity(), device).is_ok(),
            revision: snapshot.revision(),
            identity: snapshot.identity().clone(),
        });
    }
    entries.sort_by(|a, b| a.identity.session().cmp(b.identity.session()));
    Ok(entries)
}
#[cfg(test)]
#[path = "live_relay_catalog_tests.rs"]
mod tests;
