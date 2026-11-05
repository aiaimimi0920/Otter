//! 一次Tile字段编辑的共享身份/固定在途期限；只共享依赖，不拥有HWND、像素或授权租约。
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
pub(crate) struct Edit {
    pub node: String,
    pub contract: [u8; 32],
    group: Arc<Group>,
}
struct Group {
    stopped: AtomicBool,
    inflight: Mutex<Option<Instant>>,
    fields: Mutex<std::collections::BTreeMap<String, [u8; 32]>>,
}
/// 原worker的栈上清理；validate拒绝、撤权早退或panic均不能残留“HTTP仍在途”。
pub(crate) struct Dispatch(Arc<Edit>);
impl Drop for Dispatch {
    fn drop(&mut self) {
        self.0.sending(None);
    }
}
impl Edit {
    pub(crate) fn dispatch_guard(self: &Arc<Self>) -> Dispatch {
        Dispatch(self.clone())
    }
    pub fn new(node: String, contract: [u8; 32]) -> Arc<Self> {
        let fields = std::collections::BTreeMap::from([(node.clone(), contract)]);
        Arc::new(Self {
            node,
            contract,
            group: Arc::new(Group {
                stopped: AtomicBool::new(false),
                inflight: Mutex::new(None),
                fields: Mutex::new(fields),
            }),
        })
    }
    pub fn peer(node: String, contract: [u8; 32], prior: &Self) -> Arc<Self> {
        let mut fields = prior.group.fields.lock().unwrap_or_else(|e| e.into_inner());
        if fields.get(&node).is_some_and(|old| old != &contract)
            || !fields.contains_key(&node) && fields.len() >= 64
        {
            prior.stop();
        } else {
            fields.insert(node.clone(), contract);
        }
        Arc::new(Self {
            node,
            contract,
            group: prior.group.clone(),
        })
    }
    pub fn same_contract(&self, before: &serde_json::Value, after: &serde_json::Value) -> bool {
        let fields = self.group.fields.lock().unwrap_or_else(|e| e.into_inner());
        crate::tile_surface_edits::same_contracts(before, after, |id| fields.contains_key(id))
    }
    pub fn same_group(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.group, &other.group)
    }
    pub fn active(&self) -> bool {
        !self.group.stopped.load(Ordering::Acquire)
    }
    pub fn stop(&self) {
        self.group.stopped.store(true, Ordering::Release);
    }
    pub(crate) fn sending(&self, now: Option<Instant>) {
        *self
            .group
            .inflight
            .lock()
            .unwrap_or_else(|e| e.into_inner()) =
            now.map(|t| t + crate::tile_surface_actions::WAIT_LIMIT);
    }
    pub(crate) fn in_flight(&self) -> bool {
        self.in_flight_at(Instant::now())
    }
    pub(crate) fn in_flight_at(&self, now: Instant) -> bool {
        self.group
            .inflight
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some_and(|until| now < until)
    }
}
/// 新鲜已呈现字段的共同wire合同；不将文本/数字事件降格为未经校验的任意JSON。
pub(crate) fn validates(
    p: &crate::surface_presentation::Presentation,
    point: [f32; 2],
    event: &serde_json::Value,
    edit: &Edit,
) -> bool {
    use crate::surface_scene::Kind;
    let Some(node) = crate::tile_surface_value::node(p.scene().root(), &edit.node) else {
        return false;
    };
    if !edit.active()
        || !node.visible()
        || node.props().get("disabled") == Some(&serde_json::Value::Bool(true))
        || crate::surface_edit_contract::fingerprint(node).ok() != Some(edit.contract)
    {
        return false;
    }
    if !edit
        .group
        .fields
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .all(|(id, hash)| {
            p.scene()
                .root()
                .visible_node(id)
                .and_then(|n| crate::surface_edit_contract::fingerprint(n).ok())
                .as_ref()
                == Some(hash)
        })
    {
        return false;
    }
    if event["event"] == "click" {
        return p
            .click_route(point)
            .any(|a| event["nodeId"] == a.node.id() && event["action"] == a.action);
    }
    if p.hit_test(point) != Some(edit.node.as_str()) {
        return false;
    }
    let class = match event["event"].as_str() {
        Some("input") => "continuous",
        Some("change") => "commit",
        _ => return false,
    };
    if event["nodeId"] != edit.node
        || event["class"] != class
        || event.get("modifiers").is_some()
        || node
            .events()
            .get(event["event"].as_str().unwrap())
            .map(String::as_str)
            != event["action"].as_str()
        || event["payload"].as_object().is_none_or(|p| p.len() != 1)
    {
        return false;
    }
    let value = &event["payload"]["value"];
    match node.kind() {
        Kind::Slider => value.as_f64().is_some_and(|v| {
            crate::surface_slider::Range::parse(node).is_ok_and(|r| r.normalize(v) == Ok(v))
        }),
        Kind::Input | Kind::Textarea => value.as_str().is_some_and(|v| {
            v.len() <= crate::surface_input_draft::MAX_VALUE_BYTES
                && !v.contains('\0')
                && (node.kind() == Kind::Textarea || !v.contains(['\r', '\n']))
        }),
        Kind::Number => value.is_null() || value.as_f64().is_some_and(f64::is_finite),
        Kind::Switch | Kind::Select => crate::tile_surface_value::validates(p, point, event),
        _ => false,
    }
}
