//! Authenticated session-selection catalog: complete validation, disambiguation, and bounded output without credentials/observed business snapshots.
use super::*;
use serde::Deserialize;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub identity: Identity,
    pub revision: u64,
    pub member: bool,
    pub direct_view: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    protocol_version: String,
    sessions: Vec<Box<serde_json::value::RawValue>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Header {
    session: SessionHeader,
    #[serde(default)]
    direct_view: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionHeader {
    session_id: String,
}
impl Control<'_> {
    /// Lists provide selection candidates only; joining must recheck source/epoch by identity, not treat listing as join authorization.
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
            direct_view: header.direct_view,
            identity: snapshot.identity().clone(),
        });
    }
    entries.sort_by(|a, b| a.identity.session().cmp(b.identity.session()));
    Ok(entries)
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "live_relay_catalog_tests.rs"]
mod tests;
