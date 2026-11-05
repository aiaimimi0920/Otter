//! 同份公开SDK示例，原OpenArt/签名包/TLS/child/正式结果/Save恢复链；独立2x2像素。
use super::*;
#[test]
#[ignore = "requires the public service image example and independently frozen SHA-256"]
fn service_image_public_example_formal_application_save_restore_and_shutdown() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let path = PathBuf::from(std::env::var_os("OTTER_ART_IMAGE_EXAMPLE").unwrap());
    assert!(path.is_absolute());
    let source = std::fs::read_to_string(path).unwrap();
    assert_eq!(
        package::digest(source.as_bytes()),
        std::env::var("OTTER_ART_IMAGE_SHA256").unwrap()
    );
    unsafe {
        let _environment = environment();
        let root = root();
        let mut peer = package_peer::Peer::with_source(&root, &source);
        let mut app = image_app();
        authorize(&mut app, &peer, &root);
        app.art_issuers = trust(&peer);
        let original = app.documents[0]
            .state
            .canvas
            .borrow()
            .image
            .as_ref()
            .unwrap()
            .pixels
            .clone();
        start(&mut app, &peer, &root);
        pump(&mut app, "service image formal output", |app| {
            values(app).is_some()
        });
        let bytes = saved(&mut app, "image_output");
        assert!(!bytes.is_empty());
        assert!(original.same_storage(
            &app.documents[0]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .unwrap()
                .pixels
        ));
        assert!(peer.state.lock().unwrap().image_output_owner.is_none());
        app.shutdown();
        assert!(!app.local_arts.busy());
        assert!(app.authorization_owner.finished());
        peer.stop();
        let state = peer.state.lock().unwrap();
        assert_eq!(
            state
                .requests
                .iter()
                .filter(|p| p.ends_with("/submit-image-output"))
                .count(),
            1
        );
        assert_eq!(
            state
                .requests
                .iter()
                .filter(|p| p.ends_with("/cancel"))
                .count(),
            1
        );
    }
}
