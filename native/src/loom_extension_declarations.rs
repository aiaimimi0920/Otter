//! 已验证目录中的活跃声明投影；没有 socket/资源访问能力，不单独签发权限。
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
    /// 执行和附件写入仍只接受 trusted；视觉准入不构成此处的授权。
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
    /// 主干的显示 scope 包括开发模式；通知可保留，但不能据此执行命令或写附件。
    pub fn display_active(&self, scope: &str) -> bool {
        self.0.plugins.iter().any(|p| {
            p.scope_id == scope
                && matches!(p.trust_status, Trust::Trusted | Trust::UnsignedDeveloper)
        })
    }
    /// 只读场景及其命令声明存在性；真正点击仍必须经过 Snapshot::command。
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
