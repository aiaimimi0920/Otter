//! Real overlay mouse messages follow original command owners to authorized TLS invocations; no clipboard changes or model calls.
use super::*;
#[test]
fn developer_surface_is_visible_but_its_real_click_cannot_queue_an_extension_command() {
    unsafe {
        let mut data = catalog();
        data["plugins"][0]["trustStatus"] = "unsigned_developer".into();
        data["contributions"]["renderers"][0]["commandId"] = OCR.into();
        let f = Fixture::new(&data);
        let mut a = crate::unit_attachment_store::tests::attachment();
        a["schemaVersion"] = "2".into();
        a["overlayVisible"] = true.into();
        a["payload"]["surfaceScene"] = json!({"id":"root","type":"stack","layout":{"width":32,"height":20},
            "props":{"eventPayload":{"text":"developer action"}},"events":{"click":OCR}});
        a["payloadDigest"] = crate::unit_attachment_json::digest(&a["payload"]).into();
        *f.state().extension_document.borrow_mut() = extension_document::Owner::restore(
            crate::unit_attachment_store::tests::store(vec![a]),
            None,
        );
        let hwnd = f.documents[0].window.0;
        super::super::super::extension_overlay::sync(hwnd, f.state()).unwrap();
        let views = super::super::super::extension_input::views(f.state()).unwrap();
        assert!(
            views.hit([8., 10.]).is_some(),
            "developer scene must actually reach the input view"
        );
        let scale = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(96) as f32 / 96.;
        let lp = LPARAM(
            (8. * scale).round() as u16 as isize | (((10. * scale).round() as u16 as isize) << 16),
        );
        SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(lp));
        SendMessageW(hwnd, WM_LBUTTONUP, None, Some(lp));
        assert!(!busy(f.state()));
        assert!(f.owner.view.borrow().requested.is_none());
        assert!(!super::super::super::extension_overlay::available(
            f.state(),
            false
        ));
    }
}
#[test]
fn surface_click_reaches_the_same_authorized_command_with_exact_event_identity_and_payload() {
    unsafe {
        let mut data = catalog();
        data["contributions"]["renderers"][0]["commandId"] = OCR.into();
        let mut f = Fixture::new(&data);
        let mut a = crate::unit_attachment_store::tests::attachment();
        // schema 2 is not eligible for the host's schema 1 local-copy contract.
        a["schemaVersion"] = "2".into();
        a["overlayVisible"] = true.into();
        a["payload"]["surfaceScene"] = json!({"id":"root","type":"stack","layout":{"width":32,"height":20},
            "props":{"eventPayload":{"text":"declared action payload"}},"events":{"click":OCR}});
        a["payloadDigest"] = crate::unit_attachment_json::digest(&a["payload"]).into();
        *f.state().extension_document.borrow_mut() = extension_document::Owner::restore(
            crate::unit_attachment_store::tests::store(vec![a]),
            None,
        );
        let hwnd = f.documents[0].window.0;
        super::super::super::extension_overlay::sync(hwnd, f.state()).unwrap();
        let scale = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(96) as f32 / 96.;
        let lp = LPARAM(
            (8. * scale).round() as u16 as isize | (((10. * scale).round() as u16 as isize) << 16),
        );
        SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(lp));
        SendMessageW(hwnd, WM_LBUTTONUP, None, Some(lp));
        assert!(busy(f.state()));
        let expected = f
            .owner
            .view
            .borrow()
            .requested
            .as_ref()
            .unwrap()
            .input
            .clone();
        let event = &expected["surfaceEvent"];
        assert_eq!(event["attachmentId"], "neuro.official/ocr.result");
        assert_eq!(event["nodeId"], "root");
        assert_eq!(event["generation"], data["generation"]);
        assert_eq!(event["baseRevision"], 1);
        assert_eq!(event["class"], "discrete");
        assert_eq!(event["payload"], json!({"text":"declared action payload"}));
        let (identity, server) = peer::peer(move |p| {
            handshake(p, data);
            authorize(p, OCR, false);
            let r = peer::read(p);
            let params = &r["params"];
            assert_eq!(r["method"], "loom.extension.command.invoke");
            assert_eq!(params["authorizationId"], LEASE);
            assert_eq!(params["invocation"]["input"], expected);
            assert_eq!(params["invocation"]["target"]["revision"], 0);
            let id = &params["invocation"]["requestId"];
            peer::send(
                p,
                peer::response(
                    id,
                    json!({"protocol":"loom.extension.v1","apiVersion":"1.0",
                "requestId":id,"status":"succeeded","effects":[]}),
                ),
            );
        });
        f.owner.process_with(&f.documents, &Default::default(), || {
            Source::Fixed(identity)
        });
        f.finish();
        server.join().unwrap();
        assert!(f.state().error.borrow().is_none());
    }
}
