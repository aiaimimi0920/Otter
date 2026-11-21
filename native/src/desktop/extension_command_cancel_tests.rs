//! 重复触发取消通过真实 BUTTON、原 UI owner 与 TLS；不调用模型、剪贴板或全局输入。
use super::*;
use std::sync::atomic::AtomicBool;
use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;

fn cancellable_catalog() -> Value {
    let mut data = catalog();
    let command = data["contributions"]["commands"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["commandId"] == TRANSLATE)
        .unwrap();
    command["payload"]["toggleAttachmentType"] = crate::extension_visual::TRANSLATION.into();
    data["contributions"]["menus"] = json!([{
        "id":"translation-tool", "pluginId":crate::extension_visual::TRANSLATE,
        "scopeId":"scope:translation", "title":"翻译", "commandId":TRANSLATE,
        "placement":"hook.unit.toolbar"
    }]);
    data
}
fn signal(f: &Fixture) -> Arc<AtomicBool> {
    let view = f.state().extension_commands.borrow();
    view.requested.as_ref().unwrap().cancel.clone()
}
unsafe fn request_translation(f: &Fixture) {
    request_command(f.documents[0].window.0, f.state(), TRANSLATE).unwrap();
}
unsafe fn click_translation(f: &Fixture) {
    let document = &f.documents[0];
    super::super::super::tool_panel::prepare(document.panel.0, f.state()).unwrap();
    let child = GetDlgItem(
        Some(document.panel.0),
        super::super::super::edit_extension_toolbar::FIRST as i32,
    )
    .unwrap();
    assert!(
        IsWindowEnabled(child).as_bool(),
        "repeat-trigger cancellation must remain reachable"
    );
    SendMessageW(child, BM_CLICK, None, None);
    let mut message = MSG::default();
    assert!(PeekMessageW(
        &mut message,
        Some(document.window.0),
        super::super::super::tool_panel::EXECUTE,
        super::super::super::tool_panel::EXECUTE,
        PM_REMOVE
    )
    .as_bool());
    DispatchMessageW(&message);
}
fn seed_ocr(f: &Fixture) {
    f.state().extension_document.borrow_mut().store =
        crate::unit_attachment_store::tests::store(vec![ocr()]);
}
#[test]
fn second_real_button_click_cancels_the_queued_intent_without_starting_a_worker() {
    unsafe {
        let mut f = Fixture::new(&cancellable_catalog());
        click_translation(&f);
        let flag = signal(&f);
        assert!(!flag.load(Ordering::Acquire));
        click_translation(&f);
        assert!(flag.load(Ordering::Acquire));
        assert!(
            busy(f.state()),
            "cancelled work retains its slot until the owner retires it"
        );
        f.tick();
        assert!(!busy(f.state()));
        assert!(f.owner.pending.is_none());
        assert!(command_available(f.state(), TRANSLATE));
        assert!(IsWindowEnabled(
            GetDlgItem(
                Some(f.documents[0].panel.0),
                super::super::super::edit_extension_toolbar::FIRST as i32
            )
            .unwrap()
        )
        .as_bool());
    }
}
#[test]
fn real_button_cancels_blocked_authorization_and_the_original_connection_is_reclaimed() {
    unsafe {
        let data = cancellable_catalog();
        let mut f = Fixture::new(&data);
        let original = f.state().extension_document.borrow().store.stamp();
        // 用户取消时编辑条早已存在；建条/首次投影不属于 worker 中断的时限。
        super::super::super::tool_panel::prepare(f.documents[0].panel.0, f.state()).unwrap();
        let (ready, wait) = mpsc::channel();
        let (identity, server) = peer::peer(move |p| {
            handshake(p, data);
            let r = peer::read(p);
            assert_eq!(r["method"], "loom.extension.command.authorize");
            assert_eq!(r["params"]["checkOnly"], true);
            ready.send(()).unwrap();
            assert!(
                p.read().is_err(),
                "cancellation must not upload resources or invoke the model"
            );
        });
        f.begin(true, identity);
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
        let dispatch = Instant::now();
        click_translation(&f);
        eprintln!(
            "cancel timing: button_ms={}",
            dispatch.elapsed().as_millis()
        );
        assert!(!command_available(f.state(), TRANSLATE));
        assert!(f
            .owner
            .pending
            .as_ref()
            .unwrap()
            .intent
            .cancel
            .load(Ordering::Acquire));
        // 隐藏 BUTTON 的消息往返另行记录；本门限保护已接纳取消后的原 worker 回收。
        let start = Instant::now();
        f.finish();
        eprintln!("cancel timing: settled_ms={}", start.elapsed().as_millis());
        assert!(start.elapsed() < Duration::from_secs(2));
        server.join().unwrap();
        assert!(original.matches(&f.state().extension_document.borrow().store));
    }
}
fn cancel_received_result(cached_ocr: bool) {
    unsafe {
        let data = cancellable_catalog();
        let mut f = Fixture::new(&data);
        if cached_ocr {
            seed_ocr(&f);
        }
        let phase = Arc::new(AtomicUsize::new(0));
        let sent = phase.clone();
        let (release, wait) = mpsc::channel();
        let (identity, server) = peer::peer(move |p| {
            handshake(p, data);
            authorize(p, TRANSLATE, true);
            if !cached_ocr {
                authorize(p, OCR, false);
                complete(p, OCR, ocr());
            }
            authorize(p, TRANSLATE, false);
            let r = peer::read(p);
            assert_eq!(r["method"], "loom.extension.command.invoke");
            sent.store(1, Ordering::Release);
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            let id = &r["params"]["invocation"]["requestId"];
            peer::send(
                p,
                peer::response(
                    id,
                    json!({"protocol":"loom.extension.v1","apiVersion":"1.0",
                "requestId":id,"status":"succeeded","effects":[{"type":"attachment.upsert","payload":{
                    "attachmentId":"neuro.official/text-translation.result","typeId":crate::extension_visual::TRANSLATION,
                    "schemaVersion":"1","revision":1,"priorRevision":0,"resourceRefs":[],
                    "payload":{"translatedText":"must not be applied","visible":true}}}]}),
                ),
            );
            assert!(p.read().is_err());
        });
        f.begin(true, identity);
        let end = Instant::now() + Duration::from_secs(5);
        while phase.load(Ordering::Acquire) == 0 {
            f.tick();
            assert!(Instant::now() < end);
            thread::sleep(Duration::from_millis(2));
        }
        let original = f.state().extension_document.borrow().store.stamp();
        assert_eq!(f.state().extension_document.borrow().store.items().len(), 1);
        release.send(()).unwrap();
        while !f.owner.pending.as_ref().unwrap().job.request_pending() {
            assert!(Instant::now() < end);
            thread::sleep(Duration::from_millis(2));
        }
        request_translation(&f);
        f.finish();
        server.join().unwrap();
        assert!(original.matches(&f.state().extension_document.borrow().store));
    }
}
#[test]
fn cancellation_before_apply_discards_the_already_received_result_and_its_effects() {
    cancel_received_result(true);
}
#[test]
fn cancellation_preserves_ocr_already_committed_by_the_same_translation_transaction() {
    cancel_received_result(false);
}
#[test]
fn overlay_fallback_and_its_actual_command_share_the_cancellation_identity() {
    unsafe {
        let f = Fixture::new(&cancellable_catalog());
        request_translation(&f);
        let flag = signal(&f);
        request_command(
            f.documents[0].window.0,
            f.state(),
            super::super::super::extension_command_route::TRANSLATION_OVERLAY,
        )
        .unwrap();
        assert!(flag.load(Ordering::Acquire));
    }
}
#[test]
fn group_menu_remains_reachable_and_only_the_cancellable_item_is_enabled() {
    unsafe {
        use super::super::super::{edit_extension_toolbar as toolbar, tool_panel};
        let mut data = cancellable_catalog();
        data["contributions"]["menus"][0]["payload"] = json!({"groupId":"tools"});
        let mut menu = data["contributions"]["menus"][0].clone();
        menu["id"] = "other-menu".into();
        menu["commandId"] = "other-command".into();
        menu["order"] = json!(1);
        data["contributions"]["menus"]
            .as_array_mut()
            .unwrap()
            .push(menu);
        data["contributions"]["commands"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "id":"other-command","pluginId":crate::extension_visual::TRANSLATE,
                "scopeId":"scope:translation","payload":{"permissions":[]}
            }));
        let f = Fixture::new(&data);
        request_translation(&f);
        let flag = signal(&f);
        let doc = &f.documents[0];
        tool_panel::prepare(doc.panel.0, f.state()).unwrap();
        let button = GetDlgItem(Some(doc.panel.0), toolbar::FIRST as i32).unwrap();
        assert!(IsWindowEnabled(button).as_bool());
        SendMessageW(button, BM_CLICK, None, None);
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(doc.window.0),
            tool_panel::EXECUTE,
            tool_panel::EXECUTE,
            PM_REMOVE
        )
        .as_bool());
        toolbar::execute_with(
            doc.window.0,
            f.state(),
            message.wParam.0,
            |_, _, options| {
                assert_eq!(
                    options.iter().map(|o| o.enabled).collect::<Vec<_>>(),
                    [true, false]
                );
                Ok(Some(0))
            },
        )
        .unwrap();
        assert!(flag.load(Ordering::Acquire));
    }
}
#[test]
fn only_the_same_target_plugin_and_input_context_can_request_cancellation() {
    unsafe {
        let mut data = cancellable_catalog();
        let mut command = data["contributions"]["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["commandId"] == TRANSLATE)
            .unwrap()
            .clone();
        command["id"] = "translation-alias".into();
        command["commandId"] = "translation-alias".into();
        data["contributions"]["commands"]
            .as_array_mut()
            .unwrap()
            .push(command.clone());
        command["id"] = "foreign-context".into();
        command["commandId"] = "foreign-context".into();
        command["pluginId"] = crate::extension_visual::OCR.into();
        command["scopeId"] = "scope:ocr".into();
        data["contributions"]["commands"]
            .as_array_mut()
            .unwrap()
            .push(command.clone());
        command["pluginId"] = crate::extension_visual::TRANSLATE.into();
        command["scopeId"] = "scope:translation".into();
        command["id"] = "different-context".into();
        command["commandId"] = "different-context".into();
        command["payload"]["inputContext"] = "another-context".into();
        data["contributions"]["commands"]
            .as_array_mut()
            .unwrap()
            .push(command);
        let f = Fixture::new(&data);
        request_translation(&f);
        let flag = signal(&f);
        assert!(!command_available(f.state(), OCR));
        assert!(!command_available(f.state(), "foreign-context"));
        assert!(!command_available(f.state(), "different-context"));
        request_command(f.documents[0].window.0, f.state(), "different-context").unwrap();
        assert!(!flag.load(Ordering::Acquire));
        let mut other = State::new(
            Some(super::super::super::image::Image {
                width: 64,
                height: 32,
                pixels: vec![255; 64 * 32 * 4].into(),
            }),
            true,
        )
        .unwrap();
        other.extensions = f.state().extensions.clone();
        other.extension_commands = f.state().extension_commands.clone();
        assert!(!command_available(&other, TRANSLATE));
        assert!(command_available(f.state(), "translation-alias"));
        request_command(f.documents[0].window.0, f.state(), "translation-alias").unwrap();
        assert!(flag.load(Ordering::Acquire));
    }
}
#[test]
fn ordinary_non_toggle_repeat_cannot_cancel_or_replace_the_running_intent() {
    unsafe {
        let f = Fixture::new(&catalog());
        request_translation(&f);
        let flag = signal(&f);
        request_translation(&f);
        assert!(!flag.load(Ordering::Acquire));
        assert_eq!(requested_command(f.state()).as_deref(), Some(TRANSLATE));
    }
}
#[test]
fn revoked_catalog_and_changed_source_cannot_reuse_a_previous_cancel_target() {
    unsafe {
        for change_source in [false, true] {
            let mut f = Fixture::new(&cancellable_catalog());
            request_translation(&f);
            let flag = signal(&f);
            if change_source {
                let mut c = f.state().canvas.borrow_mut();
                c.editor
                    .begin(
                        crate::editor::Tool::Brush,
                        crate::viewport::Point { x: 1., y: 1. },
                    )
                    .unwrap();
                c.editor.commit();
            } else {
                let catalog = f.state().extensions.borrow().snapshot.clone().unwrap();
                f.state().extensions.borrow_mut().snapshot = Some(Arc::new(
                    Snapshot::parse(&serde_json::to_vec(&cancellable_catalog()).unwrap()).unwrap(),
                ));
                assert!(
                    catalog.same_catalog(f.state().extensions.borrow().snapshot.as_ref().unwrap())
                );
            }
            request_translation(&f);
            assert!(!flag.load(Ordering::Acquire));
            f.tick();
            assert!(!busy(f.state()));
        }
    }
}
