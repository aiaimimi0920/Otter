//! 固定来源/工作区/目录身份与当前 resolved PNG 的失败关闭测试；不联网、不占前台。
use super::*;
#[test]
fn native_sender_scope_uses_resolved_frame_and_normal_opacity_and_rejects_later_display() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, _, _) = Files::new();
    unsafe {
        let (_class, mut app) = application(files.root());
        let unit = app.documents[0].state.unit_id.clone();
        // 目标拥有逻辑框；本用例只核 resolved 来源/opacity，不额外引入不同宽高比的 contain 留白。
        app.documents[0]
            .state
            .canvas
            .borrow_mut()
            .replace_image(Image {
                width: 32,
                height: 16,
                pixels: [255, 0, 0, 255].repeat(32 * 16).into(),
            })
            .unwrap();
        let current = Image {
            width: 4,
            height: 2,
            pixels: [0, 255, 0, 255].repeat(8).into(),
        };
        *app.documents[0].state.resolved_display.borrow_mut() = Some(
            crate::desktop::display_frame::Frame::capture(unit.clone(), current, None),
        );
        app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
        let (scope, snapshot) = scope::Scope::capture(&app, &unit).unwrap();
        let png = crate::desktop::export::projection::frame(
            &snapshot.image,
            &snapshot.editor,
            snapshot.opacity,
            snapshot.kind,
            &AtomicBool::new(false),
        )
        .unwrap();
        let decoded = crate::desktop::export::decode_png(png.png.to_vec());
        assert_eq!((decoded.width, decoded.height), (4, 2));
        assert!(decoded.pixels.chunks_exact(4).all(|pixel| pixel[0] == 0
            && pixel[2] == 0
            && (127..=128).contains(&pixel[1])
            && (127..=128).contains(&pixel[3])));
        assert!(scope.matches(&app));
        app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.75;
        assert!(!scope.matches(&app));
        app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
        app.documents[0].state.resolved_display.borrow_mut().take();
        assert!(!scope.matches(&app));
        app.shutdown();
    }
}
#[test]
fn native_sender_stale_directory_never_prepares_network_and_new_unit_has_canonical_source() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, _, _) = Files::new();
    unsafe {
        let (_class, mut app) = application(files.root());
        let unit = app.documents[0].state.unit_id.clone();
        let (scope, _) = scope::Scope::capture(&app, &unit).unwrap();
        assert!(!scope.source.same_storage(&scope.binding_source));
        let value: serde_json::Value = serde_json::from_slice(&scope.binding_source).unwrap();
        assert_eq!(value["stickers"][0]["id"], unit.as_str());
        let origin = "https://loom.example";
        let body = serde_json::to_vec(&crate::projection_delivery::targets::tests::body()).unwrap();
        let directory = || {
            std::sync::Arc::new(
                crate::projection_delivery::targets::Directory::parse(&body, origin).unwrap(),
            )
        };
        app.authorization.borrow_mut().origin = Some(origin.into());
        app.authorization.borrow_mut().targets.directory = Some(directory());
        app.start_projection_sender(
            ui::sender::Selection {
                unit: unit.clone(),
                directory: directory(),
                index: 0,
            },
            &mut |_| panic!("stale directory must not prepare network"),
        );
        assert!(!app.remote_sessions.projection_pending());
        assert!(matches!(app.projection_sender.phase, Phase::Idle));
        let bytes = scope.source.to_vec();
        app.restored_session
            .as_mut()
            .unwrap()
            .edit_original(bytes.into());
        assert!(
            !scope.matches(&app),
            "equal bytes with new allocation are stale"
        );
        app.shutdown();
    }
}
#[test]
fn native_sender_cleanup_failure_has_future_deadline_and_same_intent_is_deduplicated() {
    let (directory, store) = crate::projection_delivery::sender::tests::setup();
    let intent = crate::projection_delivery::sender::tests::prepare(
        &store,
        &directory,
        &crate::projection_delivery::sender::tests::selection(1),
    )
    .unwrap();
    let request = Request::new(intent, directory.identity());
    let mut owner = cleanup::Owner::default();
    for _ in 0..=crate::projection_delivery::sender::MAX_ENTRIES {
        owner.enqueue(request.clone(), true);
    }
    owner.finished(&request, false);
    let deadline = owner.deadline().unwrap();
    assert!(deadline > Instant::now());
    owner.finished(&request, true);
    assert!(!owner.pending_local());
    assert!(
        !request.published_retired(),
        "queue removal is not durable retirement"
    );
}
#[test]
fn native_sender_receiver_role_is_rejected_before_preparing_a_signed_intent() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, _, _) = Files::new();
    unsafe {
        let (_class, mut app) = application(files.root());
        let unit = app.documents[0].state.unit_id.clone();
        let (scope, _) = scope::Scope::capture(&app, &unit).unwrap();
        let selected = crate::projection_delivery::sender::Selection::new(
            &scope.scope.to_string(),
            "https://loom.example",
            unit.as_str(),
            "receiver-one",
        )
        .unwrap();
        assert!(Binding::admit_send(&scope.binding_source, &selected).is_ok());
        let mut value: serde_json::Value = serde_json::from_slice(&scope.binding_source).unwrap();
        value["stickers"][0]["qrProjection"] = serde_json::json!({"role":"receiver"});
        assert!(
            Binding::admit_send(&serde_json::to_vec(&value).unwrap().into(), &selected).is_err()
        );
        assert!(!files.root().join("projection-sender.json").exists());
        app.shutdown();
    }
}
