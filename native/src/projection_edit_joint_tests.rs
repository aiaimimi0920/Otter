//! 显式双真实Loom消费者；只覆盖协议/写前事务，不把模拟checkpoint当Application保存验收。
use super::{
    fixture::{annotation, SESSION},
    *,
};
use crate::{
    device_identity::Identity, device_request_cancel::Cancel, device_session_attempt::Session,
    device_session_client::Client, live_relay_http::Control,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Endpoint<'a> {
    origin: String,
    certificate: String,
    #[serde(borrow)]
    identity: &'a serde_json::value::RawValue,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input<'a> {
    #[serde(borrow)]
    source: Endpoint<'a>,
    #[serde(borrow)]
    local: Endpoint<'a>,
    #[serde(borrow)]
    remote: Endpoint<'a>,
    shared: String,
    offline: String,
    digest: String,
}
struct Actor {
    client: Client,
    session: Session,
    context: journal::Context,
    profile: PathBuf,
}
impl Actor {
    fn new(root: &Path, name: &str, input: &Endpoint<'_>) -> Self {
        use base64::Engine;
        let url = url::Url::parse(&input.origin).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("127.0.0.1"));
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(rustls::pki_types::CertificateDer::from(
                base64::engine::general_purpose::STANDARD
                    .decode(&input.certificate)
                    .unwrap(),
            ))
            .unwrap();
        let client = Client::new(
            &input.origin,
            roots,
            vec![format!("127.0.0.1:{}", url.port().unwrap())
                .parse()
                .unwrap()],
        )
        .unwrap();
        let identity = Identity::parse(input.identity.get().as_bytes()).unwrap();
        let session = client
            .issue_once(&identity, &AtomicBool::new(false), |_| {})
            .unwrap();
        let context = journal::Context::new(
            serde_json::from_value(json!("a".repeat(64))).unwrap(),
            "unit-1",
            &input.origin,
            session.device_id(),
        )
        .unwrap();
        let profile = root.join(name);
        fs::create_dir(&profile).unwrap();
        Self {
            client,
            session,
            context,
            profile,
        }
    }
    fn control<'a>(&'a self, cancel: &'a Cancel) -> Control<'a> {
        Control::new(&self.client, &self.session, cancel)
    }
    fn open(&self) -> journal::Store {
        journal::Store::open(&self.profile, &AtomicBool::new(false)).unwrap()
    }
    fn begin(
        &self,
        store: &journal::Store,
        route: Route,
        request: Request,
        previous: Option<Document>,
        view: Objects,
    ) -> journal::Ticket {
        store
            .begin(
                self.context.clone(),
                route,
                request,
                previous,
                view,
                &AtomicBool::new(false),
            )
            .unwrap()
    }
}
fn apply(projection: &str, op: &str, previous: &Document, id: &str, value: Value) -> Request {
    Request::prepare(Operation::Apply {
        projection_id: projection.into(),
        session_id: SESSION.into(),
        op_id: op.into(),
        base_revision: previous.revision(),
        mode_revision: previous.mode_revision(),
        changes: vec![Change {
            object_id: id.into(),
            value,
        }],
    })
    .unwrap()
}
fn ack(ticket: &journal::Ticket, doc: Document) -> Document {
    // 专属fixture显式扮演持久checkpoint完成；产品接线尚未开放，不能据此宣称canonical保存通过。
    ticket
        .commit_after_checkpoint(doc.clone(), &AtomicBool::new(false))
        .unwrap();
    doc
}
#[test]
#[ignore = "explicit isolated two real Looms with synthetic device identities and pinned HTTPS"]
fn native_edit_joint_shared_offline_replay_merge_checkpoint_and_conflict() {
    let path = PathBuf::from(
        std::env::var_os("OTTER_PROJECTION_EDIT_JOINT_INPUT").expect("explicit isolated input"),
    );
    assert!(path.is_absolute() && fs::metadata(&path).unwrap().len() <= 64 * 1024);
    let root = path.parent().unwrap();
    assert!(root
        .canonicalize()
        .unwrap()
        .starts_with(std::env::temp_dir().canonicalize().unwrap()));
    let bytes = zeroize::Zeroizing::new(fs::read(&path).unwrap());
    let input: Input<'_> = serde_json::from_slice(&bytes).unwrap();
    let source = Actor::new(root, "source-profile", &input.source);
    let local = Actor::new(root, "local-profile", &input.local);
    let remote = Actor::new(root, "remote-profile", &input.remote);
    let cancel = Cancel::new();
    let sc = source.control(&cancel);
    let lc = local.control(&cancel);
    let rc = remote.control(&cancel);
    let ss = source.open();
    let view = BTreeMap::from([("shape-1".into(), annotation("shape-1"))]);
    let init = Request::prepare(Operation::Initialize {
        projection_id: input.shared.clone(),
        session_id: SESSION.into(),
        expected_digest: input.digest,
        objects: view.clone(),
    })
    .unwrap();
    let ticket = source.begin(&ss, Route::Shared, init, None, view);
    let original = ticket.request().bytes().to_vec();
    let first = sc.projection_edit(&ticket).unwrap();
    assert_eq!(first.revision(), 1);
    // 已成功但尚未确认的写前记录在重新open后仍重放同一请求，服务端revision不能增加。
    drop((ticket, ss));
    let ss = source.open();
    let mut pending = ss.pending(&AtomicBool::new(false)).unwrap();
    assert_eq!(pending.len(), 1);
    let ticket = pending.pop().unwrap();
    assert_eq!(ticket.request().bytes(), original);
    let first = ack(&ticket, sc.projection_edit(&ticket).unwrap());
    assert_eq!(first.revision(), 1);
    drop(ticket);
    let attach = Request::prepare(Operation::Attach {
        projection_id: input.offline.clone(),
        session_id: SESSION.into(),
    })
    .unwrap();
    let ticket = source.begin(
        &ss,
        Route::Offline,
        attach,
        Some(first.clone()),
        first.live_objects(),
    );
    let attached = ack(&ticket, sc.projection_edit(&ticket).unwrap());
    assert_eq!(attached, first);
    drop(ticket);
    let mode = Request::prepare(Operation::Mode {
        projection_id: input.shared.clone(),
        session_id: SESSION.into(),
        op_id: "native-enable".into(),
        base_mode_revision: 1,
        mode: Mode::TwoWay,
    })
    .unwrap();
    let ticket = source.begin(
        &ss,
        Route::Shared,
        mode,
        Some(first.clone()),
        first.live_objects(),
    );
    let second = ack(&ticket, sc.projection_edit(&ticket).unwrap());
    assert_eq!(second.revision(), 2);
    drop(ticket);
    let remote_base = rc
        .read_projection_edit(&remote.context, &input.offline, Route::Offline, None)
        .unwrap();
    let local_base = lc
        .read_projection_edit(&local.context, &input.shared, Route::Shared, None)
        .unwrap();
    assert_eq!(remote_base, local_base);
    let rs = remote.open();
    let ls = local.open();
    let mut value = annotation("shape-1");
    value["x"] = 10.into();
    let mut remote_view = remote_base.live_objects();
    remote_view.insert("shape-1".into(), value.clone());
    let remote_ticket = remote.begin(
        &rs,
        Route::Offline,
        apply(
            &input.offline,
            "native-remote",
            &remote_base,
            "shape-1",
            value,
        ),
        Some(remote_base),
        remote_view,
    );
    assert_eq!(rc.projection_edit(&remote_ticket).unwrap().revision(), 3);
    let mut local_view = local_base.live_objects();
    let mut value = annotation("local");
    value["x"] = 20.into();
    local_view.insert("local".into(), value.clone());
    let local_ticket = local.begin(
        &ls,
        Route::Shared,
        apply(&input.shared, "native-local", &local_base, "local", value),
        Some(local_base),
        local_view,
    );
    let fourth = ack(&local_ticket, lc.projection_edit(&local_ticket).unwrap());
    assert_eq!(fourth.revision(), 4);
    let replayed = ack(&remote_ticket, rc.projection_edit(&remote_ticket).unwrap());
    assert_eq!(replayed, fourth);
    ss.observe(&source.context, fourth.clone(), &AtomicBool::new(false))
        .unwrap();
    let checkpoint = Request::prepare(Operation::Checkpoint {
        projection_id: input.shared.clone(),
        session_id: SESSION.into(),
        expected_revision: 4,
    })
    .unwrap();
    let ticket = source.begin(
        &ss,
        Route::Shared,
        checkpoint,
        Some(fourth.clone()),
        fourth.live_objects(),
    );
    let fifth = ack(&ticket, sc.projection_edit(&ticket).unwrap());
    assert_eq!(
        (
            fifth.revision(),
            fifth.mode_revision(),
            fifth.checkpoint_revision(),
            fifth.receipt_count()
        ),
        (5, 5, 5, 0)
    );
    rs.observe(&remote.context, fifth.clone(), &AtomicBool::new(false))
        .unwrap();
    let mut conflict_view = fifth.live_objects();
    let mut conflict = annotation("shape-1");
    conflict["x"] = 99.into();
    conflict_view.insert("shape-1".into(), conflict.clone());
    let conflict_ticket = source.begin(
        &ss,
        Route::Shared,
        apply(
            &input.shared,
            "native-conflict",
            &fifth,
            "shape-1",
            conflict,
        ),
        Some(fifth.clone()),
        conflict_view,
    );
    let mut deleted = fifth.live_objects();
    deleted.remove("shape-1");
    let deletion = remote.begin(
        &rs,
        Route::Offline,
        apply(
            &input.offline,
            "native-delete",
            &fifth,
            "shape-1",
            Value::Null,
        ),
        Some(fifth),
        deleted,
    );
    let sixth = ack(&deletion, rc.projection_edit(&deletion).unwrap());
    assert_eq!(sixth.revision(), 6);
    let before = fs::read(source.profile.join("projection-edit.json")).unwrap();
    assert_eq!(
        sc.projection_edit(&conflict_ticket),
        Err(Failure::Rejected {
            status: 409,
            code: Code::ObjectConflict
        })
    );
    assert_eq!(
        fs::read(source.profile.join("projection-edit.json")).unwrap(),
        before
    );
    drop((ticket, conflict_ticket, ss));
    let reopened = source.open();
    let pending = reopened.pending(&AtomicBool::new(false)).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].view()["shape-1"]["x"], 99);
    assert_eq!(
        sc.projection_edit(&pending[0]),
        Err(Failure::Rejected {
            status: 409,
            code: Code::ObjectConflict
        })
    );
    let shared = lc
        .read_projection_edit(&local.context, &input.shared, Route::Shared, Some(&fourth))
        .unwrap();
    let offline = rc
        .read_projection_edit(
            &remote.context,
            &input.offline,
            Route::Offline,
            Some(&sixth),
        )
        .unwrap();
    assert_eq!(shared, offline);
    assert_eq!(offline, sixth);
    let output = json!({"document":offline,"actualHttps":true,"actualLoomServices":2,"sameRequestReplayed":true,"conflictRetained":true,"applicationCheckpoint":false,"peerTls":false});
    let staging = root.join("completed.pending");
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .unwrap();
    file.write_all(&serde_json::to_vec(&output).unwrap())
        .unwrap();
    file.sync_all().unwrap();
    drop(file);
    fs::rename(staging, root.join("completed.json")).unwrap();
}
