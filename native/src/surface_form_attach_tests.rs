//! Production report→TLS attach→owned HWND→authorized batch→TLS snapshot→revocation.
//! Lifecycle activation ACK is injected; no external service or desktop input injection.
use super::*;
#[path = "surface_form_attach_peer.rs"]
mod peer;
use crate::desktop::surface_authorized_window::AuthorizedWindow;
use windows::{
    core::{w, HSTRING},
    Win32::{
        Foundation::*,
        UI::{Controls::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
struct Apartment;
impl Apartment {
    fn new() -> Self {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
            )
            .ok()
            .unwrap();
        }
        Self
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}

fn activate(window: &mut AuthorizedWindow) {
    let mut owner = window.begin_activation(0).unwrap().unwrap();
    let state = crate::surface_live_binding::Lifecycle::Active;
    let revision = owner.begin(state).unwrap();
    let binding = owner.binding();
    let ack = serde_json::to_vec(&serde_json::json!({"descriptor":{
        "instanceId":binding.instance(),"attachmentId":binding.attachment(),
        "hookNodeId":binding.node(),"deviceId":binding.device()},
        "lifecycle":"active","lifecycleRevision":revision}))
    .unwrap();
    owner.acknowledge(&ack, state, revision).unwrap();
    assert!(window.finish_activation(owner).is_ok());
    window.show(0).unwrap();
    assert!(window.sync_input(true, true, 0).unwrap());
}
unsafe fn edit(window: &AuthorizedWindow, kind: &str) -> HWND {
    let parent = window.edit_test_parent();
    if kind == "switch" {
        for message in [WM_LBUTTONDOWN, WM_LBUTTONUP] {
            SendMessageW(parent, message, None, Some(LPARAM(10 | (9 << 16))));
        }
        return parent;
    }
    let class = match kind {
        "select" => w!("COMBOBOX"),
        "slider" => w!("STATIC"),
        _ => w!("EDIT"),
    };
    let child = FindWindowExW(Some(parent), None, class, None).unwrap();
    // Slider intentionally ignores keys addressed to an unfocused control.
    let _ = SetFocus(Some(child));
    assert_eq!(GetFocus(), child);
    if matches!(kind, "select" | "slider") {
        let key = if kind == "slider" { VK_RIGHT } else { VK_DOWN };
        SendMessageW(child, WM_KEYDOWN, Some(WPARAM(key.0 as usize)), None);
        SendMessageW(child, WM_KEYUP, Some(WPARAM(key.0 as usize)), None);
    } else {
        SendMessageW(child, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        let value = HSTRING::from(if kind == "number" { "42" } else { "中文🙂" });
        SendMessageW(
            child,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(value.as_ptr() as isize)),
        );
    }
    child
}
#[test]
fn native_report_forms_attach_edit_submit_apply_and_revoke_through_tls() {
    let _com = Apartment::new();
    for kind in ["input", "textarea", "number", "switch", "select", "slider"] {
        let (listener, client, config) = fixture();
        let origin = client.origin.clone();
        let mut grant = Grant::new(session(&origin, u64::MAX));
        let lease = grant.lease(&origin, 0).unwrap();
        let report = crate::desktop::surface_capabilities::report().unwrap();
        let wire = serde_json::to_value(&report).unwrap();
        let peer = std::thread::spawn(move || peer::serve(kind, listener, config, wire));
        let data = Arc::new(
            AttachRequest::new(
                "art:1".into(),
                "node:1".into(),
                report,
                Persistence::Temporary,
            )
            .unwrap(),
        );
        let mut attach =
            Job::start_prepared(client.clone(), grant.lease(&origin, 0).unwrap(), data).unwrap();
        wait(&attach);
        let outcome = attach.try_join().unwrap();
        assert!(outcome.result.is_ok(), "{kind}: {:?}", outcome.result);
        let mut window =
            AuthorizedWindow::new(&outcome, grant.lease(&origin, 0).unwrap(), 0, [240, 160])
                .unwrap();
        activate(&mut window);
        let child = unsafe { edit(&window, kind) };
        assert!(window.has_pending_click(), "{kind}");
        let events = window.take_prepared_click(0).unwrap().unwrap();
        assert_eq!(events.len(), 1, "{kind}");
        let owner = window.begin_batch(&events, 0).unwrap().unwrap();
        let input = crate::surface_event_batch_runtime::Input {
            owner,
            revision: 3,
            events,
        };
        let mut batch =
            crate::surface_event_batch_runtime::Job::start_prepared(client, lease, input)
                .unwrap_or_else(|error| panic!("{kind}: {}", error.error));
        assert_eq!(
            unsafe {
                WaitForSingleObject(HANDLE(batch.wait_handle().unwrap().as_raw_handle()), 5000)
            },
            WAIT_OBJECT_0
        );
        let delivered = batch.try_join(0).unwrap();
        let receipt = window
            .accept_batch(delivered, 0)
            .unwrap_or_else(|_| panic!("{kind}: rejected owner"));
        assert!(
            matches!(receipt.presentation, Ok(Some(4))),
            "{kind}: {:?}",
            receipt.presentation
        );
        assert_eq!(window.snapshot_revision(), 4);
        assert!(window.lifecycle().unwrap().confirmed_active());
        // The authoritative response removes the field, not merely echoes its local value.
        if kind != "switch" {
            assert!(
                !unsafe { IsWindow(Some(child)) }.as_bool(),
                "{kind}: old field survived"
            );
        }
        assert!(window.sync_input(true, true, 0).unwrap());
        unsafe {
            let parent = window.edit_test_parent();
            for message in [WM_LBUTTONDOWN, WM_LBUTTONUP] {
                SendMessageW(parent, message, None, Some(LPARAM(10 | (9 << 16))));
            }
        }
        assert!(
            window
                .take_prepared_click(0)
                .unwrap()
                .unwrap_or_default()
                .is_empty(),
            "{kind}: removed field still submits"
        );
        peer.join().unwrap();
        drop(grant);
        window.refresh(1).unwrap();
        assert!(!window.is_alive());
        assert!(!unsafe { IsWindow(Some(child)) }.as_bool());
        assert!(!window.lifecycle().unwrap().disposed());
    }
}
