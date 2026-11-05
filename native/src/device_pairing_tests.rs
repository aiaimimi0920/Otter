use super::*;

#[test]
fn request_preserves_public_key_and_main_wire_shape_without_private_material() {
    let identity = Identity::generate().unwrap();
    let request = Request::new(&identity, " 中文电脑 ").unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
        serde_json::json!({"name":"中文电脑", "kind":"computer", "address":"hook://中文电脑",
            "publicKey":identity.public_key().unwrap()})
    );
    assert_eq!(identity.device_id(), None);
    for name in ["", " ", "line\nnext", "tab\tname", "nul\0name"] {
        assert!(matches!(Request::new(&identity, name), Err(Error::Name)));
    }
    assert!(matches!(
        Request::new(&identity, &"x".repeat(257)),
        Err(Error::Name)
    ));
}

#[test]
fn registry_matches_only_own_public_key_and_rejects_ambiguous_ids() {
    let identity = Identity::generate().unwrap();
    let key = identity.public_key().unwrap();
    for field in ["pending", "devices"] {
        let body = serde_json::to_vec(&serde_json::json!({field:[
            {"id":"unrelated","publicKey":"other"}, {"id":"device-1","publicKey":key}]}))
        .unwrap();
        assert_eq!(
            Request::new(&identity, "PC").unwrap().finish(201, &body),
            Ok("device-1".into())
        );
    }
    let duplicate = |id| {
        serde_json::to_vec(&serde_json::json!({
        "pending":[{"id":"device-1","publicKey":key}],
        "devices":[{"id":id,"publicKey":key}]}))
        .unwrap()
    };
    assert_eq!(
        Request::new(&identity, "PC")
            .unwrap()
            .finish(200, &duplicate("device-1")),
        Ok("device-1".into())
    );
    assert_eq!(
        Request::new(&identity, "PC")
            .unwrap()
            .finish(200, &duplicate("device-2")),
        Err(Error::Ambiguous)
    );
    for id in ["", "bad\nidentifier"] {
        assert_eq!(
            Request::new(&identity, "PC")
                .unwrap()
                .finish(200, &duplicate(id)),
            Err(Error::Device)
        );
    }
}

#[test]
fn malformed_unmatched_oversized_and_failed_registry_responses_never_bind() {
    let identity = Identity::generate().unwrap();
    for (body, expected) in [
        (b"{}".as_slice(), Error::Missing),
        (b"bad json", Error::Json),
        (br#"{"pending":[],"pending":[]}"#, Error::Json),
        (br#"{"devices":[{"id":"other"}]}"#, Error::Missing),
    ] {
        assert_eq!(
            Request::new(&identity, "PC").unwrap().finish(200, body),
            Err(expected)
        );
    }
    assert_eq!(
        Request::new(&identity, "PC")
            .unwrap()
            .finish(403, b"sensitive"),
        Err(Error::Http(403))
    );
    assert_eq!(
        Request::new(&identity, "PC").unwrap().finish(
            200,
            &vec![0; crate::device_session_attempt::MAX_RESPONSE + 1]
        ),
        Err(Error::Limit)
    );
    assert_eq!(identity.device_id(), None);
}
