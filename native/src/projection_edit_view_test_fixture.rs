//! projectionEditProtocol既有120x100 fixture的独立常量；不调用映射器生成预期。
use super::*;
use crate::projection_edit::fixture::{DIGEST, SESSION};
use serde_json::json;
pub(super) fn basis() -> Basis {
    Basis::new(DIGEST, 120, 100).unwrap()
}
pub(super) fn rect(id: &str) -> Value {
    json!({"id":id,"type":"rect","zIndex":1.25,"style":{"color":"#123456","width":2},
        "x":10,"y":20,"w":30,"h":25})
}
pub(super) fn objects() -> Objects {
    Objects::from([("shape-a".into(), rect("shape-a"))])
}
pub(super) fn view() -> View {
    View::from_basis(&basis(), Size::new(120.0, 100.0).unwrap(), objects()).unwrap()
}
pub(super) fn document(objects: Objects, revision: u64) -> Document {
    let objects: std::collections::BTreeMap<_, _> = objects
        .into_iter()
        .map(|(id, value)| (id, json!({"revision":revision,"value":value})))
        .collect();
    parse_document(
        json!({"schema":"neuro.projection-edit.v1","sessionId":SESSION,
        "basis":{"digest":DIGEST,"width":120,"height":100},"revision":revision,
        "modeRevision":1,"checkpointRevision":1,"receiptCount":revision-1,"mode":"two_way","objects":objects}),
    )
}
pub(super) fn parse_document(value: Value) -> Document {
    Document::parse(&serde_json::to_vec(&value).unwrap()).unwrap()
}
pub(super) fn move_x(view: &View, id: &str, x: f64) -> View {
    let mut local = view.local_objects().unwrap();
    local.get_mut(id).unwrap()["x"] = x.into();
    view.edited(local).unwrap()
}
