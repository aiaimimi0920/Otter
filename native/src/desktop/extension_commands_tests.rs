//! 隐藏真实宿主→首次 OCR/翻译→TLS/WS→附件 CAS→原 Canvas；不访问真实 Loom/系统剪贴板。
use super::*;
use crate::{
    desktop::extension_command_route::{OCR, TRANSLATE},
    loom_extension_session::test_peer as peer,
};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
const LEASE: &str = "extension-auth:01234567-89ab-4cde-8123-0123456789ab";
#[path = "extension_command_cancel_tests.rs"]
mod cancellation;
#[path = "extension_command_surface_tests.rs"]
mod surface;
struct Fixture {
    owner: Owner,
    documents: Vec<DocumentWindow>,
    _apartment: super::super::Apartment,
}
impl Fixture {
    unsafe fn new(data: &Value) -> Self {
        let apartment = super::super::Apartment::new().unwrap();
        let instance = HINSTANCE(
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .unwrap()
                .0,
        );
        RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: windows::core::w!("Otter.Native.Workspace"),
            lpfnWndProc: Some(super::super::procedure::window_proc),
            ..Default::default()
        });
        let state = State::new(
            Some(super::super::image::Image {
                width: 64,
                height: 32,
                pixels: vec![255; 64 * 32 * 4].into(),
            }),
            true,
        )
        .unwrap();
        state.extensions.borrow_mut().snapshot = Some(Arc::new(
            Snapshot::parse(&serde_json::to_vec(data).unwrap()).unwrap(),
        ));
        extension_document::observe(&state);
        let owner = Owner::new(state.extension_commands.clone(), Arc::new(|| {}));
        let document = DocumentWindow::create(instance, state, false).unwrap();
        Self {
            owner,
            documents: vec![document],
            _apartment: apartment,
        }
    }
    fn state(&self) -> &State {
        &self.documents[0].state
    }
    unsafe fn begin(&mut self, translation: bool, identity: crate::loom_bridge_identity::Identity) {
        request_command(
            self.documents[0].window.0,
            self.state(),
            if translation { TRANSLATE } else { OCR },
        )
        .unwrap();
        assert!(busy(self.state()));
        self.owner
            .process_with(&self.documents, &Default::default(), || {
                Source::Fixed(identity)
            });
    }
    unsafe fn begin_button(
        &mut self,
        translation: bool,
        identity: crate::loom_bridge_identity::Identity,
    ) {
        let document = &self.documents[0];
        super::super::tool_panel::prepare(document.panel.0, &document.state).unwrap();
        document
            .state
            .canvas
            .borrow_mut()
            .select_tool(crate::editor::Tool::Brush);
        // 真实编辑条 OCR 控件 ID；不注入全局鼠标/键盘。
        SendMessageW(
            GetDlgItem(
                Some(document.panel.0),
                if translation { 0x6109 } else { 0x6108 },
            )
            .unwrap(),
            BM_CLICK,
            None,
            None,
        );
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(document.window.0),
            super::super::tool_panel::EXECUTE,
            super::super::tool_panel::EXECUTE,
            PM_REMOVE
        )
        .as_bool());
        DispatchMessageW(&message);
        assert_eq!(
            self.state().canvas.borrow().tool,
            crate::editor::Tool::Select
        );
        assert!(self.state().canvas.borrow().selection_enabled);
        assert!(busy(self.state()));
        self.owner
            .process_with(&self.documents, &Default::default(), || {
                Source::Fixed(identity)
            });
    }
    unsafe fn tick(&mut self) {
        self.owner
            .process_with(&self.documents, &Default::default(), || {
                panic!("unexpected retry")
            });
    }
    unsafe fn finish(&mut self) {
        let end = Instant::now() + Duration::from_secs(8);
        while self.owner.pending.is_some() {
            self.tick();
            assert!(Instant::now() < end, "command did not finish");
            thread::sleep(Duration::from_millis(2));
        }
        assert!(!busy(self.state()));
    }
}
fn catalog() -> Value {
    let mut v = crate::extension_visual::tests::catalog_value();
    let permissions = json!([
        "hook.unit.image.read",
        "hook.unit.attachments.read",
        "hook.unit.attachments.write",
        "hook.notice.show"
    ]);
    v["plugins"][0]["effectivePermissions"] = permissions.clone();
    let run = json!({"id":OCR,"commandId":OCR,"pluginId":crate::extension_visual::OCR,"scopeId":"scope:ocr",
        "when":"unit.kind == 'sticker' && unit.hasImage","payload":{"permissions":permissions}});
    let translate = json!({"id":TRANSLATE,"commandId":TRANSLATE,"pluginId":crate::extension_visual::TRANSLATE,"scopeId":"scope:translation",
        "when":"unit.kind == 'sticker' && unit.hasImage","payload":{"inputContext":"ocr-text.v1","permissions":["hook.unit.attachments.read","hook.unit.attachments.write"]}});
    v["contributions"]["commands"]
        .as_array_mut()
        .unwrap()
        .extend([run, translate]);
    v
}
fn handshake(peer: &mut peer::Peer, data: Value) {
    let mut h = peer::handshake_data();
    h["snapshot"] = data;
    h["features"]
        .as_array_mut()
        .unwrap()
        .push("ocr-text.v1".into());
    peer::handshake(peer, h);
}
fn authorize(peer: &mut peer::Peer, command: &str, check: bool) {
    let r = peer::read(peer);
    let p = &r["params"];
    assert_eq!(r["method"], "loom.extension.command.authorize");
    assert_eq!(p["commandId"], command);
    assert_eq!(p["checkOnly"], check);
    assert_eq!(p["target"]["revision"], 0);
    peer::send(
        peer,
        peer::response(
            &p["requestId"],
            if check {
                json!({"authorized":true})
            } else {
                json!({"authorizationId":LEASE})
            },
        ),
    );
}
fn complete(peer: &mut peer::Peer, command: &str, payload: Value) -> Value {
    let r = peer::read(peer);
    assert_eq!(r["method"], "loom.extension.command.invoke");
    assert_eq!(r["params"]["invocation"]["commandId"], command);
    assert_eq!(r["params"]["authorizationId"], LEASE);
    let id = &r["params"]["invocation"]["requestId"];
    peer::send(
        peer,
        peer::response(
            id,
            json!({"protocol":"loom.extension.v1","apiVersion":"1.0","requestId":id,
        "status":"succeeded","effects":[{"type":"attachment.upsert","payload":payload},
            {"type":"notice.show","payload":{"title":"OCR 已完成","message":"合成测试结果"}}]}),
        ),
    );
    r
}
fn ocr() -> Value {
    let mut value = crate::unit_attachment_store::tests::attachment();
    value["priorRevision"] = 0.into();
    value["payload"] = json!({"fullText":"示例","sourceWidth":64,"sourceHeight":32,"textBlocks":[],"visible":true,
        "surfaceScene":{"id":"ocr-result","type":"text","props":{"text":"示例"}}});
    value
}
#[test]
fn first_ocr_prepares_real_png_after_authorization_and_commits_into_the_canvas() {
    unsafe {
        let data = catalog();
        let mut f = Fixture::new(&data);
        let doc = f.state().canvas.borrow().editor.document_stamp();
        assert!(super::super::extension_overlay::available(f.state(), false));
        let (identity, server) = peer::peer(move |p| {
            handshake(p, data);
            authorize(p, super::super::extension_command_route::OCR_OVERLAY, true);
            authorize(p, OCR, false);
            let r = complete(p, OCR, ocr());
            use base64::Engine;
            let png = base64::engine::general_purpose::STANDARD
                .decode(
                    r["params"]["resourceUploads"][0]["dataBase64"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap();
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
            assert_eq!(&png[16..24], &[0, 0, 0, 64, 0, 0, 0, 32]);
        });
        f.begin_button(false, identity);
        f.finish();
        server.join().unwrap();
        assert_eq!(
            f.state().extension_document.borrow().store.items()[0]
                .attachment
                .payload()
                .unwrap()["fullText"],
            "示例"
        );
        let canvas = f.state().canvas.borrow();
        assert!(canvas
            .extension_views
            .as_ref()
            .unwrap()
            .active(&canvas, None, false));
        assert!(doc.matches(&canvas.editor));
    }
}
#[path = "extension_command_cache_tests.rs"]
mod cached;
#[test]
fn closing_target_while_authorizing_cancels_without_a_late_result() {
    unsafe {
        let data = catalog();
        let mut f = Fixture::new(&data);
        let (ready, wait) = mpsc::channel();
        let (identity, server) = peer::peer(move |p| {
            handshake(p, data);
            let r = peer::read(p);
            assert_eq!(r["method"], "loom.extension.command.authorize");
            ready.send(()).unwrap();
            assert!(p.read().is_err());
        });
        f.begin(false, identity);
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
        DestroyWindow(f.documents[0].window.0).unwrap();
        f.finish();
        server.join().unwrap();
        assert!(f
            .state()
            .extension_document
            .borrow()
            .store
            .items()
            .is_empty());
    }
}
#[test]
fn attachment_change_after_upload_rejects_the_old_inflight_effects() {
    unsafe {
        let data = catalog();
        let mut f = Fixture::new(&data);
        let phase = Arc::new(AtomicUsize::new(0));
        let sent = phase.clone();
        let (identity, server) = peer::peer(move |p| {
            handshake(p, data);
            authorize(p, OCR, false);
            assert_eq!(peer::read(p)["method"], "loom.extension.command.invoke");
            sent.store(1, Ordering::Release);
            assert!(p.read().is_err());
        });
        f.begin(false, identity);
        let end = Instant::now() + Duration::from_secs(5);
        while phase.load(Ordering::Acquire) == 0 {
            f.tick();
            assert!(Instant::now() < end);
            thread::sleep(Duration::from_millis(2));
        }
        f.state().extension_document.borrow_mut().store =
            crate::unit_attachment_store::tests::store(vec![
                crate::unit_attachment_store::tests::attachment(),
            ]);
        let stamp = f.state().extension_document.borrow().store.stamp();
        f.finish();
        server.join().unwrap();
        assert!(stamp.matches(&f.state().extension_document.borrow().store));
    }
}
#[test]
fn first_translation_runs_ocr_then_exports_only_named_context_to_the_translation_owner() {
    unsafe {
        let data = catalog();
        let mut f = Fixture::new(&data);
        let (identity, server) = peer::peer(move |p| {
            handshake(p, data);
            authorize(p, TRANSLATE, true);
            authorize(p, OCR, false);
            complete(p, OCR, ocr());
            authorize(p, TRANSLATE, false);
            let r = peer::read(p);
            let call = &r["params"]["invocation"];
            let input = &call["input"];
            assert_eq!(call["commandId"], TRANSLATE);
            assert_eq!(input["text"], "示例");
            assert_eq!(input["targetLanguage"], "zh-CN");
            assert_eq!(input["sourceRevision"], 0);
            assert!(input["sourceAttachment"]["digest"].as_str().is_some());
            assert!(
                call.get("unitAttachments").is_none(),
                "foreign opaque OCR attachment leaked"
            );
            assert!(r["params"].get("resourceUploads").is_none());
            let id = &call["requestId"];
            peer::send(
                p,
                peer::response(
                    id,
                    json!({"protocol":"loom.extension.v1","apiVersion":"1.0","requestId":id,"status":"succeeded",
                "effects":[{"type":"attachment.upsert","payload":{"attachmentId":"neuro.official/text-translation.result",
                    "typeId":crate::extension_visual::TRANSLATION,"schemaVersion":"1","revision":1,"priorRevision":0,"resourceRefs":[],
                    "payload":{"visible":true,"sourceRevision":0,"sourceAttachment":input["sourceAttachment"],"originalText":"示例",
                        "surfaceScene":{"id":"translation","type":"text","props":{"text":"Example"}}}}}]}),
                ),
            );
        });
        f.begin(true, identity);
        f.finish();
        server.join().unwrap();
        assert_eq!(f.state().extension_document.borrow().store.items().len(), 2);
        let c = f.state().canvas.borrow();
        assert!(c.extension_views.as_ref().unwrap().active(&c, None, true));
    }
}
#[test]
fn same_generation_different_catalog_is_rejected_before_authorization_or_resources() {
    unsafe {
        let data = catalog();
        let mut f = Fixture::new(&data);
        let (identity, server) = peer::peer(move |p| {
            let mut changed = data;
            changed["plugins"][0]["packageDigest"] = "c".repeat(64).into();
            handshake(p, changed);
            assert!(p.read().is_err());
        });
        f.begin(false, identity);
        f.finish();
        server.join().unwrap();
        assert!(f
            .state()
            .extension_document
            .borrow()
            .store
            .items()
            .is_empty());
    }
}
#[test]
fn changing_source_or_revoking_catalog_during_authorization_cancels_without_writing_results() {
    unsafe {
        for revoke in [false, true] {
            let data = catalog();
            let mut f = Fixture::new(&data);
            let (ready, wait) = mpsc::channel();
            let (identity, server) = peer::peer(move |p| {
                handshake(p, data);
                let r = peer::read(p);
                assert_eq!(r["method"], "loom.extension.command.authorize");
                ready.send(()).unwrap();
                assert!(p.read().is_err());
            });
            f.begin(false, identity);
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            if revoke {
                f.state().extensions.borrow_mut().snapshot = None;
            } else {
                let mut c = f.state().canvas.borrow_mut();
                c.editor
                    .begin(
                        crate::editor::Tool::Brush,
                        crate::viewport::Point { x: 1., y: 1. },
                    )
                    .unwrap();
                c.editor.commit();
            }
            f.finish();
            server.join().unwrap();
            assert!(f
                .state()
                .extension_document
                .borrow()
                .store
                .items()
                .is_empty());
        }
    }
}
#[test]
fn shutdown_wakes_resource_rendezvous_without_ui_polling_or_leaking_worker() {
    unsafe {
        let data = catalog();
        let mut f = Fixture::new(&data);
        let done = Arc::new(AtomicUsize::new(0));
        let signal = done.clone();
        let (identity, server) = peer::peer(move |p| {
            handshake(p, data);
            authorize(p, OCR, false);
            signal.store(1, Ordering::Release);
            assert!(p.read().is_err());
        });
        f.begin(false, identity);
        let end = Instant::now() + Duration::from_secs(5);
        while done.load(Ordering::Acquire) == 0 {
            assert!(Instant::now() < end);
            thread::sleep(Duration::from_millis(2));
        }
        thread::sleep(Duration::from_millis(20));
        let start = Instant::now();
        f.owner.shutdown();
        assert!(start.elapsed() < Duration::from_secs(2));
        server.join().unwrap();
        assert!(f
            .state()
            .extension_document
            .borrow()
            .store
            .items()
            .is_empty());
    }
}
