//! 正式结果提交后的 Art 通知；可经图片钉图中继，不以预览触发，不在此联网。
use super::Application;
use crate::{loom_capability::Capability, workspace_id::UnitId};
impl Application {
    /// 新建连接复用原通知链；从 Art 出发仍必须尊重源 notify，不制造手动执行。
    pub(super) unsafe fn propagate_created_child(&self, source: &UnitId) {
        if self
            .documents
            .iter()
            .any(|doc| doc.state.unit_id == *source && doc.state.is_art())
        {
            self.propagate_result(source);
        } else {
            self.propagate_to_targets(source);
        }
    }
    fn notification_targets(
        &self,
        source: &UnitId,
    ) -> Result<Vec<UnitId>, crate::graph_image::Error> {
        use crate::graph_image::{Graph, Kind, Node, Output, Port};
        let results: Vec<_> = self
            .documents
            .iter()
            .map(|doc| doc.state.execution_results.borrow())
            .collect();
        let values: Vec<Vec<_>> = results
            .iter()
            .map(|result| {
                result
                    .ports()
                    .iter()
                    .map(|port| {
                        (
                            port.name.as_str(),
                            match &port.value {
                                crate::desktop::loom_output_batch::Output::Image(_)
                                | crate::desktop::loom_output_batch::Output::PngImage(_) => {
                                    Output::Image("owned-formal-image")
                                }
                                crate::desktop::loom_output_batch::Output::Value(_) => Output::Null,
                            },
                        )
                    })
                    .collect()
            })
            .collect();
        let ports = [Port {
            name: "image",
            kind: "image",
        }];
        let mut nodes: Vec<_> = self
            .documents
            .iter()
            .enumerate()
            .map(|(index, doc)| {
                let art = doc.state.is_art();
                let canvas = doc.state.canvas.borrow();
                Node {
                    id: &doc.state.unit_id,
                    kind: if art { Kind::Art } else { Kind::Sticker },
                    inputs: &ports,
                    outputs: &[],
                    capability_inputs: art.then_some(&[][..]),
                    capability_outputs: &[],
                    values: if art { &values[index] } else { &[] },
                    source: (!art && canvas.image.is_some()).then_some("owned-local-image"),
                    preview: None,
                    manual_image: None,
                    input_disabled: doc.state.sticker_input.snapshot().disabled,
                    accept_upstream: doc.state.accept_upstream.get(),
                    locally_edited: canvas.editor.has_edit_state(),
                    pending: false,
                }
            })
            .collect();
        for session in self
            .live_sessions
            .iter()
            .filter(|session| session.is_alive() && session.belongs_to(&self.workspace))
        {
            nodes.push(Node {
                id: session.unit_id(),
                kind: Kind::Sticker,
                inputs: &[],
                outputs: &[],
                capability_inputs: None,
                capability_outputs: &[],
                values: &[],
                source: Some("owned-live-image"),
                preview: None,
                manual_image: None,
                input_disabled: false,
                accept_upstream: false,
                locally_edited: false,
                pending: false,
            });
        }
        Ok(Graph::new(&nodes, self.workspace.links())?
            .notification_targets(source)?
            .into_iter()
            .cloned()
            .collect())
    }
    pub(super) unsafe fn propagate_result(&self, source: &UnitId) {
        let Some(origin) = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == *source)
        else {
            return;
        };
        let selected = origin.state.selected_art.borrow();
        let Some(selected) = selected.as_ref() else {
            return;
        };
        let notify = selected
            .catalog
            .handshake
            .arts()
            .iter()
            .filter_map(Capability::new)
            .find(|cap| cap.id == selected.parameters.art_id())
            .is_some_and(|_| selected.execution.notify_downstream);
        if !notify {
            return;
        }
        self.propagate_to_targets(source);
    }
    pub(super) unsafe fn propagate_live_snapshot(&self, source: &UnitId) {
        if self.live_sessions.iter().any(|session| {
            session.unit_id() == source && session.is_alive() && session.belongs_to(&self.workspace)
        }) {
            self.propagate_to_targets(source);
        }
    }
    unsafe fn propagate_to_targets(&self, source: &UnitId) {
        let Ok(ancestors) = self.workspace.upstream_units(source) else {
            return;
        };
        let Ok(targets) = self.notification_targets(source) else {
            return;
        };
        for document in &self.documents {
            let state = &document.state;
            if state.group_locked.get()
                || ancestors.contains(&&state.unit_id)
                || !targets.contains(&state.unit_id)
            {
                continue;
            }
            let selected = state.selected_art.borrow();
            let Some(selected) = selected.as_ref() else {
                continue;
            };
            let accepts = selected
                .catalog
                .handshake
                .arts()
                .iter()
                .filter_map(Capability::new)
                .find(|cap| cap.id == selected.parameters.art_id())
                .is_some_and(|cap| {
                    let config = selected.execution;
                    cap.enabled
                        && config.accepts_upstream_notification()
                        && config
                            .gate(
                                crate::graph_execution_config::Trigger::Upstream,
                                cap.runtime(),
                            )
                            .is_ok()
                });
            if !accepts {
                continue;
            }
            // 捕获旧执行取消令牌后才排入新通知，旧结果不会在新请求前重新覆盖。
            if super::super::execution_cancel::request(document.window.0, state).is_ok() {
                super::super::execution_start::upstream_changed(state);
            }
        }
    }
}
#[cfg(test)]
impl Application {
    pub(super) unsafe fn verify_direct_propagation(&mut self) {
        use crate::desktop::{
            capability_selection::Selection, document_state::new_unit_id, execution_cancel,
            execution_start,
        };
        use crate::workspace::Link;
        let source = self.documents[0].state.unit_id.clone();
        let child = &self.documents[2].state;
        let child_id = child.unit_id.clone();
        let origin = self.documents[0].state.selected_art.borrow();
        let origin = origin.as_ref().unwrap();
        *child.selected_art.borrow_mut() = Some(Selection {
            execution: Default::default(),
            identity: new_unit_id().unwrap(),
            catalog: origin.catalog.clone(),
            parameters: origin.parameters.clone(),
        });
        self.workspace
            .connect(Link {
                id: new_unit_id().unwrap(),
                from_unit: source.clone(),
                from_port: "output".into(),
                to_unit: child_id.clone(),
                to_port: "amount".into(),
            })
            .unwrap();
        self.propagate_result(&source);
        assert_eq!(
            child.pending_start.borrow().as_ref().unwrap().trigger,
            crate::graph_execution_config::Trigger::Upstream
        );
        // 重复边/通知合并在同一个槽；显式手动操作优先级不会被后台通知降低。
        *child.pending_start.borrow_mut() = Some(execution_start::Request {
            stamp: crate::desktop::configuration_stamp::Stamp::capture(child),
            trigger: crate::graph_execution_config::Trigger::Manual,
        });
        self.propagate_result(&source);
        assert_eq!(
            child.pending_start.borrow().as_ref().unwrap().trigger,
            crate::graph_execution_config::Trigger::Manual
        );
        execution_cancel::stop(self.documents[2].window.0, child).unwrap();
        child.group_locked.set(true);
        self.propagate_result(&source);
        assert!(child.pending_start.borrow().is_none());
        child.group_locked.set(false);
        // 导入接口允许已有环，自动通知必须终止而不是触发无尽网络执行。
        self.workspace
            .add_link(Link {
                id: new_unit_id().unwrap(),
                from_unit: child_id,
                from_port: "output".into(),
                to_unit: source.clone(),
                to_port: "cycle".into(),
            })
            .unwrap();
        self.propagate_result(&source);
        assert!(child.pending_start.borrow().is_none());
        assert!(self.input_jobs.is_empty() && self.execution_jobs.is_empty());
    }
}
