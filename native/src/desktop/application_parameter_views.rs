//! 在版本变化的 UI 调度段生成有效参数视图，复用执行图解析，不回写手动值。
use super::Application;
use crate::{desktop::parameter_view::View, loom_capability::Capability};
use serde_json::Map;
impl Application {
    pub(super) fn sync_parameter_views(&self) {
        for doc in &self.documents {
            let selected = doc.state.selected_art.borrow();
            let Some(selected) = selected.as_ref() else {
                doc.state.parameter_view.borrow_mut().take();
                continue;
            };
            let Some(capability) = selected
                .catalog
                .handshake
                .arts()
                .iter()
                .filter_map(Capability::new)
                .find(|capability| capability.id == selected.parameters.art_id())
            else {
                doc.state.parameter_view.borrow_mut().take();
                continue;
            };
            let linked: Vec<String> =
                capability
                    .parameters()
                    .filter(|p| {
                        !p.secret
                            && self.workspace.links().iter().any(|link| {
                                link.to_unit == doc.state.unit_id && link.to_port == p.id
                            })
                    })
                    .map(|p| p.id.into())
                    .collect();
            // 普通手动参数由菜单直接借用，尤其不要为文件 Data URL 再保留完整副本。
            if linked.is_empty() {
                View::update(&doc.state, Map::new(), linked);
                continue;
            }
            let resolved = Self::with_parameters(
                &self.documents,
                &self.workspace,
                &doc.state.unit_id,
                &capability,
                &selected.parameters,
                |resolved| {
                    crate::desktop::parameter_view_values::capture(
                        resolved
                            .entries()
                            .filter(|(name, _)| linked.iter().any(|id| id == name)),
                    )
                },
            );
            // 来源暂不可用也保留连线锁定，不能把解析失败变成手动编辑准入。
            View::update(
                &doc.state,
                resolved.map(|(values, _)| values).unwrap_or_default(),
                linked,
            );
        }
    }
}
