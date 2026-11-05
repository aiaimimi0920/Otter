use super::*;
#[test]
fn candidate_remount_rechecks_identity_generation_art_and_runtime() {
    for failure in ["", "generation", "art", "runtime", "attachment"] {
        let (listener, client, config) = fixture();
        let session = session(&client.origin, u64::MAX);
        let peer = std::thread::spawn(move || {
            let mut tls = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut tls);
            assert!(head.starts_with("GET /v1/surfaces/stream?"));
            reply(
                &mut tls,
                200,
                &serde_json::to_vec(&stream(serde_json::json!([snapshot(
                    "art:1",
                    "surface:test/part"
                )])))
                .unwrap(),
            );
            drop(tls);
            let mut tls =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, body) = request(&mut tls);
            assert!(head.starts_with("POST /v1/surfaces/instances/surface:test%2Fpart/mount "));
            assert_eq!(body, serde_json::json!({"attachmentId":"attachment:1"}));
            let mut result = mounted();
            result["instance"]["descriptor"]["artId"] = "art:1".into();
            result["instance"]["attachments"]["attachment:1"]["snapshot"]["artId"] = "art:1".into();
            match failure {
                "generation" => result["instance"]["descriptor"]["generation"] = 1.into(),
                "art" => result["instance"]["descriptor"]["artId"] = "other".into(),
                "runtime" => {
                    result["instance"]["attachments"]["attachment:1"]["snapshot"]["runtime"] =
                        "javascript".into()
                }
                "attachment" => {
                    result["instance"]["attachments"]["attachment:1"]["snapshot"]["attachmentId"] =
                        "other".into()
                }
                _ => (),
            }
            reply(&mut tls, 200, &serde_json::to_vec(&result).unwrap());
        });
        let cancel = Cancel::new();
        let control = Control::new(&client, &session, &cancel);
        let candidate = control
            .surface_recovery_candidate("art:1", "node:1")
            .unwrap()
            .unwrap();
        let report = crate::surface_host_capabilities::Report::new(
            vec![crate::surface_host_capabilities::Runtime::Declarative],
            vec!["text".into()],
            vec![],
            vec![],
            Default::default(),
        )
        .unwrap();
        let result = control.remount_surface_candidate(&candidate, &report);
        if failure.is_empty() {
            let result = result.unwrap();
            assert_eq!(result.binding.lifecycle_revision(), 7);
            assert_eq!(result.binding.generation(), 2);
        } else {
            assert!(matches!(result, Err(LiveError::SurfaceBinding(_))));
        }
        peer.join().unwrap();
    }
}
fn snapshot(art: &str, instance: &str) -> serde_json::Value {
    serde_json::json!({"method":"loom.surface.snapshot","params":{"hookNodeId":"node:1","generation":2,
        "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":instance,"attachmentId":"attachment:1","artId":art,"revision":3}}})
}
fn stream(messages: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"protocolVersion":"loom.surface-stream.v1","next":10,"reset":false,"messages":messages})
}
#[test]
fn recovery_is_device_authenticated_read_only_and_deduplicates_identity() {
    let (listener, client, config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut tls = StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, _) = request(&mut tls);
        assert!(head.starts_with("GET /v1/surfaces/stream?after=0&timeoutMs=0 "));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device fixture-secret"));
        assert!(head.to_ascii_lowercase().contains("x-loom-device-nonce:"));
        let message = snapshot("art:1", "surface:1");
        reply(
            &mut tls,
            200,
            &serde_json::to_vec(&stream(serde_json::json!([message, message]))).unwrap(),
        );
    });
    let cancel = Cancel::new();
    let result = Control::new(&client, &session, &cancel)
        .surface_recovery_candidate("art:1", "node:1")
        .unwrap()
        .unwrap();
    assert_eq!(result.instance(), "surface:1");
    assert_eq!(result.device(), "device-1");
    assert_eq!(result.origin(), client.origin);
    assert_eq!(result.generation(), 2);
    peer.join().unwrap();
}
#[test]
fn recovery_absence_is_unknown_and_conflicting_bindings_are_not_guessed() {
    for (messages, expected) in [
        (serde_json::json!([]), None),
        (
            serde_json::json!([snapshot("other", "surface:1")]),
            Some(crate::surface_live_binding::Error::Identity),
        ),
        (
            serde_json::json!([
                snapshot("art:1", "surface:1"),
                snapshot("art:1", "surface:2")
            ]),
            Some(crate::surface_live_binding::Error::Ambiguous),
        ),
    ] {
        let (listener, client, config) = fixture();
        let session = session(&client.origin, u64::MAX);
        let peer = std::thread::spawn(move || {
            let mut tls =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut tls);
            reply(
                &mut tls,
                200,
                &serde_json::to_vec(&stream(messages)).unwrap(),
            );
        });
        let cancel = Cancel::new();
        let result =
            Control::new(&client, &session, &cancel).surface_recovery_candidate("art:1", "node:1");
        match expected {
            None => assert_eq!(result.unwrap(), None),
            Some(error) => assert_eq!(result, Err(LiveError::SurfaceBinding(error))),
        }
        peer.join().unwrap();
    }
}
