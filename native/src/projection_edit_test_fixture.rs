//! 独立固定v1 wire夹具；预期文档不经过被测Request或合并实现生成。
use super::*;
use serde_json::{json, Value};
pub(crate) const PROJECTION: &str = "projection:11111111111111111111111111111111";
pub(crate) const SESSION: &str = "edit:22222222222222222222222222222222";
pub(crate) const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(crate) fn annotation(id: &str) -> Value {
    json!({"id":id,"type":"rect","zIndex":0,"style":{"color":"#123456","width":2},
        "x":3,"y":4,"w":10,"h":20})
}
pub(crate) fn view() -> Objects {
    BTreeMap::from([("shape-1".into(), annotation("shape-1"))])
}
pub(crate) fn document_value() -> Value {
    json!({"schema":"neuro.projection-edit.v1","sessionId":SESSION,
        "basis":{"digest":DIGEST,"width":32,"height":16},"revision":1,"modeRevision":1,
        "checkpointRevision":1,"receiptCount":0,"mode":"one_way",
        "objects":{"shape-1":{"revision":1,"value":annotation("shape-1")}}})
}
pub(crate) fn document() -> Document {
    Document::parse(&serde_json::to_vec(&document_value()).unwrap()).unwrap()
}
pub(crate) fn initialize() -> Request {
    Request::prepare(Operation::Initialize {
        projection_id: PROJECTION.into(),
        session_id: SESSION.into(),
        expected_digest: DIGEST.into(),
        objects: view(),
    })
    .unwrap()
}
pub(crate) fn mode() -> Request {
    Request::prepare(Operation::Mode {
        projection_id: PROJECTION.into(),
        session_id: SESSION.into(),
        op_id: "mode-original".into(),
        base_mode_revision: 1,
        mode: Mode::TwoWay,
    })
    .unwrap()
}
pub(crate) fn mode_document() -> Document {
    let mut v = document_value();
    v["revision"] = 2.into();
    v["modeRevision"] = 2.into();
    v["receiptCount"] = 1.into();
    v["mode"] = "two_way".into();
    Document::parse(&serde_json::to_vec(&v).unwrap()).unwrap()
}
#[cfg(windows)]
pub(crate) struct Directory(pub(crate) std::path::PathBuf);
#[cfg(windows)]
impl Directory {
    pub(crate) fn new() -> Self {
        let root=std::path::Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-parallel-20261029/main-projection-edit-1/tmp");
        std::fs::create_dir_all(root).unwrap();
        let path = root.join(format!("edit-{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub(crate) fn path(&self) -> std::path::PathBuf {
        self.0.join("projection-edit.json")
    }
    pub(crate) fn store(&self) -> journal::Store {
        journal::Store::open(&self.0, &std::sync::atomic::AtomicBool::new(false)).unwrap()
    }
}
#[cfg(windows)]
pub(crate) fn context(origin: &str) -> journal::Context {
    journal::Context::new(
        serde_json::from_value(json!("a".repeat(64))).unwrap(),
        "unit-1",
        origin,
        "device-1",
    )
    .unwrap()
}
