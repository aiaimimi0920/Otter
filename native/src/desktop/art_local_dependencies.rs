//! 本地Art的输入拓扑凭证；只保留有界身份/连线，不持像素，不因无关节点编辑退休。
use crate::{
    workspace::{IdentityStamp, Link, Workspace},
    workspace_id::UnitId,
};

pub(super) struct Stamp {
    identity: IdentityStamp,
    target: UnitId,
    upstream: Vec<UnitId>,
    links: Vec<Link>,
}
impl Stamp {
    pub(super) fn capture(workspace: &Workspace, target: &UnitId) -> Result<Self, &'static str> {
        let upstream = workspace
            .upstream_units(target)
            .map_err(|_| "Art 输入图无效")?;
        let mut units = Vec::new();
        units
            .try_reserve_exact(upstream.len())
            .map_err(|_| "无法保留 Art 输入身份")?;
        units.extend(upstream.into_iter().cloned());
        let count = workspace
            .links()
            .iter()
            .filter(|link| units.contains(&link.to_unit))
            .count();
        let mut links = Vec::new();
        links
            .try_reserve_exact(count)
            .map_err(|_| "无法保留 Art 输入连接")?;
        links.extend(
            workspace
                .links()
                .iter()
                .filter(|link| units.contains(&link.to_unit))
                .cloned(),
        );
        Ok(Self {
            identity: workspace.identity_stamp(),
            target: target.clone(),
            upstream: units,
            links,
        })
    }
    pub(super) fn matches(&self, workspace: &Workspace) -> bool {
        if !self.identity.matches(workspace)
            || !workspace.contains(&self.target)
            || self.upstream.iter().any(|unit| !workspace.contains(unit))
        {
            return false;
        }
        // 新边若使更多上游可达，必先进入原可达集合；不必在UI轮询中重新分配遍历队列。
        let mut current = workspace
            .links()
            .iter()
            .filter(|link| self.upstream.contains(&link.to_unit));
        self.links
            .iter()
            .all(|expected| current.next() == Some(expected))
            && current.next().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(value: &str) -> UnitId {
        UnitId::parse(value).unwrap()
    }
    fn graph() -> Workspace {
        let mut workspace = Workspace::with_initial(id("source"));
        workspace
            .add_units([id("middle"), id("target"), id("other")])
            .unwrap();
        workspace
            .connect(Link {
                id: id("a"),
                from_unit: id("source"),
                from_port: "output".into(),
                to_unit: id("middle"),
                to_port: "input".into(),
            })
            .unwrap();
        workspace
            .connect(Link {
                id: id("b"),
                from_unit: id("middle"),
                from_port: "output".into(),
                to_unit: id("target"),
                to_port: "input_image".into(),
            })
            .unwrap();
        workspace
    }
    #[test]
    fn unrelated_nodes_and_outgoing_links_do_not_retire_the_input_owner() {
        let mut workspace = graph();
        let stamp = Stamp::capture(&workspace, &id("target")).unwrap();
        workspace.add_units([id("new")]).unwrap();
        workspace
            .connect(Link {
                id: id("unrelated"),
                from_unit: id("target"),
                from_port: "output".into(),
                to_unit: id("new"),
                to_port: "input".into(),
            })
            .unwrap();
        assert!(stamp.matches(&workspace));
    }
    #[test]
    fn new_transitive_input_and_replaced_workspace_reject_old_execution() {
        let mut workspace = graph();
        let stamp = Stamp::capture(&workspace, &id("target")).unwrap();
        assert!(stamp.matches(&workspace.clone()));
        assert!(!stamp.matches(&graph()));
        workspace
            .connect(Link {
                id: id("changed"),
                from_unit: id("other"),
                from_port: "output".into(),
                to_unit: id("middle"),
                to_port: "other".into(),
            })
            .unwrap();
        assert!(!stamp.matches(&workspace));
    }
    #[test]
    fn removed_upstream_or_target_and_unknown_target_are_rejected() {
        let mut workspace = graph();
        let stamp = Stamp::capture(&workspace, &id("target")).unwrap();
        workspace.retain_units(|unit| unit != &id("source"));
        assert!(!stamp.matches(&workspace));
        assert!(Stamp::capture(&workspace, &id("missing")).is_err());
    }
}
