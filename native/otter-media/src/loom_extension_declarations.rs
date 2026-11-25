//! Active-declaration projection from a verified catalog; no socket/resource access and no independent permission issuance.
use super::*;
pub struct Owner<'a> {
    pub plugin: &'a str,
    pub version: &'a str,
}
pub struct Declaration<'a> {
    pub id: &'a str,
    pub plugin: &'a str,
    pub scope: &'a str,
    pub command: Option<&'a str>,
    pub payload: Option<&'a Value>,
}
impl Snapshot {
    /// Execution and attachment writes still accept only trusted; visual admission does not constitute authorization here.
    pub fn owner(&self, scope: &str) -> Option<Owner<'_>> {
        self.0
            .plugins
            .iter()
            .find(|p| p.scope_id == scope && p.trust_status == Trust::Trusted)
            .map(|p| Owner {
                plugin: &p.id,
                version: &p.version,
            })
    }
    pub fn declares(&self, scope: &str, category: &str, id: &str) -> bool {
        self.declarations(category)
            .any(|d| d.scope == scope && d.id == id)
    }
    pub fn declarations<'a>(&'a self, category: &'a str) -> impl Iterator<Item = Declaration<'a>> {
        self.raw_declarations(category)
            .filter(|d| self.owner(d.scope).is_some())
    }
    /// Mainline display scope includes development mode; notifications may remain, but cannot authorize commands or attachment writes.
    pub fn display_active(&self, scope: &str) -> bool {
        self.0.plugins.iter().any(|p| {
            p.scope_id == scope
                && matches!(p.trust_status, Trust::Trusted | Trust::UnsignedDeveloper)
        })
    }
    /// Read only whether a scene and its command declaration exist; actual clicks must still pass through Snapshot::command.
    pub fn visual_declarations<'a>(
        &'a self,
        category: &'a str,
    ) -> impl Iterator<Item = Declaration<'a>> {
        self.raw_declarations(category).filter(|d| {
            self.0.plugins.iter().any(|p| {
                p.id == d.plugin
                    && p.scope_id == d.scope
                    && matches!(p.trust_status, Trust::Trusted | Trust::UnsignedDeveloper)
            })
        })
    }
    fn raw_declarations<'a>(&'a self, category: &'a str) -> impl Iterator<Item = Declaration<'a>> {
        self.0
            .contributions
            .get(category)
            .into_iter()
            .flatten()
            .map(|item| Declaration {
                id: &item.id,
                plugin: &item.plugin_id,
                scope: &item.scope_id,
                command: item.command_id.as_deref(),
                payload: item.payload.as_ref(),
            })
    }
}
