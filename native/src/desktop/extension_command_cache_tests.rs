//! Real button to remote checkOnly to cache visibility; authorization rejection/source changes leave attachments unchanged, with no image upload/model calls.
use super::*;
use crate::desktop::extension_command_route::{OCR_OVERLAY, TRANSLATION_OVERLAY};
unsafe fn cached() -> Fixture {
    let f = Fixture::new(&catalog());
    let propagation = serde_json::value::RawValue::from_string("{\"revision\":4}".into()).unwrap();
    *f.state().extension_document.borrow_mut() = extension_document::Owner::restore(
        crate::extension_visual::tests::attachments(),
        Some(&propagation),
    );
    crate::desktop::extension_overlay::sync(f.documents[0].window.0, f.state()).unwrap();
    f
}
fn check(peer: &mut peer::Peer, command: &str, allowed: bool) {
    let r = peer::read(peer);
    assert_eq!(r["method"], "loom.extension.command.authorize");
    assert_eq!(r["params"]["checkOnly"], true);
    assert_eq!(r["params"]["commandId"], command);
    assert_eq!(r["params"]["target"]["revision"], 4);
    peer::send(
        peer,
        peer::response(&r["params"]["requestId"], json!({"authorized":allowed})),
    );
}
#[test]
fn cached_buttons_only_change_presentation_after_remote_check_and_preserve_source_and_payload() {
    unsafe {
        let mut f = cached();
        let original = f.state().extension_document.borrow().store.clone();
        let doc = f.state().canvas.borrow().editor.document_stamp();
        for (translation, command) in [(true, TRANSLATION_OVERLAY), (false, OCR_OVERLAY)] {
            let (identity, server) = peer::peer(move |p| {
                handshake(p, catalog());
                check(p, command, true);
                assert!(
                    p.read().is_err(),
                    "cache toggle invoked a model or uploaded resources"
                );
            });
            f.begin_button(translation, identity);
            f.finish();
            server.join().unwrap();
            let c = f.state().canvas.borrow();
            assert!(c
                .extension_views
                .as_ref()
                .unwrap()
                .active(&c, None, translation));
        }
        let owner = f.state().extension_document.borrow();
        assert_eq!(owner.revision(), Some(4));
        for (before, after) in original.items().iter().zip(owner.store.items()) {
            assert_eq!(before.attachment.payload(), after.attachment.payload());
            assert_eq!(before.attachment.revision(), after.attachment.revision());
        }
        assert!(doc.matches(&f.state().canvas.borrow().editor));
    }
}
#[test]
fn remote_denial_and_source_change_while_checking_never_toggle_the_cached_result() {
    unsafe {
        for change in [false, true] {
            let mut f = cached();
            let stamp = f.state().extension_document.borrow().store.stamp();
            let (ready, wait) = mpsc::channel();
            let (identity, server) = peer::peer(move |p| {
                handshake(p, catalog());
                if change {
                    let _ = peer::read(p);
                    ready.send(()).unwrap();
                    assert!(p.read().is_err());
                } else {
                    check(p, OCR_OVERLAY, false);
                }
            });
            f.begin_button(false, identity);
            if change {
                wait.recv_timeout(Duration::from_secs(5)).unwrap();
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
            assert!(stamp.matches(&f.state().extension_document.borrow().store));
        }
    }
}
