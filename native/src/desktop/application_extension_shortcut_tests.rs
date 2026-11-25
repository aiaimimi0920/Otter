//! Original Application local/global messages -> sole execution owner; the global fixture does not occupy the everyday product's Ctrl+4.
use super::*;
use crate::{desktop::capability_job::Source, loom_extension_session::test_peer as peer};
use serde_json::{json, Value};
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
struct Keys([u8; 256]);
impl Keys {
    unsafe fn control() -> Self {
        let mut old = [0; 256];
        GetKeyboardState(&mut old).unwrap();
        let mut held = [0; 256];
        held[VK_CONTROL.0 as usize] = 0x80;
        SetKeyboardState(&held).unwrap();
        Self(old)
    }
}
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe {
            SetKeyboardState(&self.0).unwrap();
        }
    }
}
fn data(global: bool) -> Value {
    let mut v = crate::extension_visual::tests::catalog_value();
    v["plugins"][0]["effectivePermissions"] = json!([
        "hook.unit.image.read",
        "hook.unit.attachments.read",
        "hook.unit.attachments.write"
    ]);
    v["contributions"]["commands"].as_array_mut().unwrap().push(json!({"id":"run-ocr",
        "commandId":desktop::extension_command_route::OCR,"pluginId":crate::extension_visual::OCR,"scopeId":"scope:ocr",
        "when":"unit.hasImage","payload":{"permissions":["hook.unit.image.read","hook.unit.attachments.write"]}}));
    v["contributions"]["shortcuts"] = json!([{"id":"recognize-shortcut","pluginId":crate::extension_visual::OCR,
        "scopeId":"scope:ocr","commandId":desktop::extension_command_route::OCR,"when":"unit.hasImage",
        "payload":{"keys":if global { "Ctrl+Alt+Shift+F11" } else { "Ctrl+4" },"global":global}}]);
    v
}
unsafe fn app(global: bool) -> Application {
    let mut app = super::super::chrome::tests::app();
    for _ in 0..2 {
        app.add_state(
            State::new(
                Some(crate::image_decode::Image {
                    width: 2,
                    height: 2,
                    pixels: vec![255; 16].into(),
                }),
                true,
            )
            .unwrap(),
            false,
        )
        .unwrap();
    }
    app.extensions.view.borrow_mut().snapshot = Some(Arc::new(
        crate::loom_extension_snapshot::Snapshot::parse(
            &serde_json::to_vec(&data(global)).unwrap(),
        )
        .unwrap(),
    ));
    app.sync_extension_shortcuts();
    app
}
#[test]
fn local_shortcuts_respect_text_input_repeat_capture_and_the_actual_message_owner() {
    unsafe {
        let _apartment = desktop::Apartment::new().unwrap();
        let _keys = Keys::control();
        let mut app = app(false);
        let host = app.documents[2].window.0;
        let child = desktop::Window(
            CreateWindowExW(
                Default::default(),
                w!("Edit"),
                w!("draft"),
                WS_CHILD,
                0,
                0,
                20,
                20,
                Some(host),
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let key = |hwnd, repeat| MSG {
            hwnd,
            message: WM_KEYDOWN,
            wParam: WPARAM(b'4' as usize),
            lParam: LPARAM(if repeat { 1 << 30 } else { 0 }),
            ..Default::default()
        };
        let mut unbound = key(host, false);
        unbound.wParam = WPARAM(b'5' as usize);
        assert!(!app.translate_extension_shortcuts(&unbound));
        assert!(!app.translate_extension_shortcuts(&key(child.0, false)));
        assert!(!app.translate_extension_shortcuts(&key(child.0, true)));
        assert!(app.translate_extension_shortcuts(&key(host, true)));
        assert!(!desktop::extension_commands::busy(&app.documents[2].state));
        app.capture_active.set(true);
        assert!(!app.translate_extension_shortcuts(&key(host, false)));
        app.capture_active.set(false);
        assert!(app.translate_extension_shortcuts(&key(host, false)));
        assert_eq!(
            app.extension_shortcuts.target.borrow().as_ref(),
            Some(&app.documents[2].state.unit_id)
        );
        assert!(desktop::extension_commands::busy(&app.documents[2].state));
        drop(child);
        app.shutdown();
    }
}
#[test]
fn registered_global_message_executes_for_last_selected_unit_not_first_window_and_closes_its_registration(
) {
    unsafe {
        let _apartment = desktop::Apartment::new().unwrap();
        let mut app = app(true);
        let target = app.documents[2].state.unit_id.clone();
        SendMessageW(app.documents[2].window.0, WM_MOUSEACTIVATE, None, None);
        assert_eq!(
            app.extension_shortcuts.target.borrow().as_ref(),
            Some(&target)
        );
        let message = MSG {
            message: WM_HOTKEY,
            wParam: WPARAM(0x5900),
            lParam: LPARAM(((VK_F11.0 as u32) << 16 | 7) as isize),
            ..Default::default()
        };
        assert!(app.translate_extension_shortcuts(&message));
        assert!(desktop::extension_commands::busy(&app.documents[2].state));
        let target_id = target.as_str().to_owned();
        let (identity, server) = peer::peer(move |p| {
            let mut hello = peer::handshake_data();
            hello["snapshot"] = data(true);
            peer::handshake(p, hello);
            let r = peer::read(p);
            assert_eq!(r["method"], "loom.extension.command.authorize");
            assert_eq!(r["params"]["target"]["unitId"], target_id);
            assert_eq!(r["params"]["checkOnly"], false);
            peer::send(
                p,
                peer::response(
                    &r["params"]["requestId"],
                    json!({"authorizationId":"extension-auth:01234567-89ab-4cde-8123-0123456789ab"}),
                ),
            );
            let r = peer::read(p);
            assert_eq!(r["method"], "loom.extension.command.invoke");
            assert!(r["params"]["resourceUploads"].is_array());
            let id = &r["params"]["invocation"]["requestId"];
            let mut attachment = crate::unit_attachment_store::tests::attachment();
            attachment["priorRevision"] = 0.into();
            peer::send(
                p,
                peer::response(
                    id,
                    json!({"protocol":"loom.extension.v1","apiVersion":"1.0","requestId":id,"status":"succeeded",
                "effects":[{"type":"attachment.upsert","payload":attachment}]}),
                ),
            );
        });
        app.extension_commands
            .process_with(&app.documents, &Default::default(), || {
                Source::Fixed(identity)
            });
        let end = Instant::now() + Duration::from_secs(8);
        while desktop::extension_commands::busy(&app.documents[2].state) {
            app.extension_commands
                .process_with(&app.documents, &Default::default(), || {
                    panic!("unexpected replay")
                });
            assert!(Instant::now() < end);
            thread::sleep(Duration::from_millis(2));
        }
        server.join().unwrap();
        assert_eq!(
            app.documents[2]
                .state
                .extension_document
                .borrow()
                .store
                .items()
                .len(),
            1
        );
        assert!(app.documents[..2].iter().all(|d| d
            .state
            .extension_document
            .borrow()
            .store
            .items()
            .is_empty()));
        DestroyWindow(app.documents[2].window.0).unwrap();
        assert!(app.translate_extension_shortcuts(&message));
        assert!(!desktop::extension_commands::busy(&app.documents[1].state));
        app.extensions.shutdown();
        app.sync_extension_shortcuts();
        assert!(!app.translate_extension_shortcuts(&message));
        app.shutdown();
    }
}
