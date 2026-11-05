//! 自有离屏HWND的真实down/up与注入选择回调；不打开模态窗、不抢焦点或操作用户窗口。
use super::*;
use crate::tile_surface_actions::tests::{ack_value, confirmation, value, Fixture};
use windows::Win32::System::Com::*;
struct Apartment;
impl Apartment {
    fn new() -> Self {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        }
        Self
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
fn window(f: &Fixture) -> Window {
    let mut o = crate::tile_tests::output();
    (o.x, o.y) = (-30000, -30000);
    let w = Window::new(f.shared.clone(), &o).unwrap();
    w.tick().unwrap();
    w.tick().unwrap();
    assert!(w.state.painted.get());
    w
}
fn queued(f: &Fixture) -> bool {
    f.shared
        .lock()
        .actions
        .take(&f.state, Instant::now())
        .is_some()
}
unsafe fn message(w: &Window, msg: u32, x: i32, y: i32) {
    SendMessageW(
        w.hwnd,
        msg,
        Some(WPARAM(0)),
        Some(LPARAM(x as u16 as isize | ((y as u16 as isize) << 16))),
    );
}
#[test]
fn real_down_up_requires_same_painted_hit_and_clears_on_focus_hidden_or_snapshot_change() {
    let _com = Apartment::new();
    let f = Fixture::new("art");
    let w = window(&f);
    unsafe {
        message(&w, WM_LBUTTONUP, 0, 0);
    }
    assert!(!queued(&f));
    w.test_art_click([0, 0]);
    assert!(queued(&f));
    unsafe {
        message(&w, WM_LBUTTONDOWN, 0, 0);
        message(&w, WM_LBUTTONUP, 2, 0);
    }
    assert!(!queued(&f));
    unsafe {
        message(&w, WM_LBUTTONDOWN, 0, 0);
        message(&w, WM_KILLFOCUS, 0, 0);
        message(&w, WM_LBUTTONUP, 0, 0);
    }
    assert!(!queued(&f));
    unsafe {
        message(&w, WM_LBUTTONDOWN, 0, 0);
        let _ = ShowWindow(w.hwnd, SW_HIDE);
        message(&w, WM_LBUTTONUP, 0, 0);
        let _ = ShowWindow(w.hwnd, SW_SHOWNOACTIVATE);
        message(&w, WM_LBUTTONUP, 0, 0);
    }
    assert!(!queued(&f));
    unsafe {
        message(&w, WM_LBUTTONDOWN, 0, 0);
    }
    w.state.painted.set(false);
    unsafe {
        message(&w, WM_LBUTTONUP, 0, 0);
    }
    assert!(!queued(&f));
    assert_ne!(unsafe { GetForegroundWindow() }, w.hwnd);
}
#[test]
fn frozen_identifying_dpi_and_revoked_owners_never_submit_old_clicks() {
    let _com = Apartment::new();
    for case in 0..4 {
        let f = Fixture::new("art");
        let w = window(&f);
        unsafe {
            message(&w, WM_LBUTTONDOWN, 0, 0);
        }
        match case {
            0 => f.shared.lock().frozen = true,
            1 => {
                let t = Instant::now();
                let endpoint =
                    crate::tile_wire::Endpoint::image("device", &crate::tile_tests::output());
                f.shared.lock().identification.observe(
                    Some(&crate::tile_wire::Identification {
                        request_id: "11111111-1111-4111-8111-111111111111".into(),
                        remaining_ms: 1000,
                        applied: false,
                    }),
                    &endpoint,
                    "lease",
                    t,
                    t,
                );
            }
            2 => unsafe {
                message(&w, WM_DPICHANGED, 0, 0);
            },
            _ => f.shared.stop(),
        }
        unsafe {
            message(&w, WM_LBUTTONUP, 0, 0);
        }
        assert!(!queued(&f));
    }
}
fn waiting(f: &Fixture) -> crate::tile_surface_activity::Confirmation {
    let c = f.commands().pop().unwrap();
    let Kind::Event(event) = &c.kind else {
        panic!()
    };
    let id = event["eventId"].as_str().unwrap();
    let ack = crate::surface_action_ack::Ack::parse_identity(
        &serde_json::to_vec(&ack_value(id, "awaiting_confirmation")).unwrap(),
        "art",
        id,
    )
    .unwrap();
    let mut v = value(&f.job.key, "art");
    v["confirmations"] = serde_json::json!([confirmation(id, &f.job.key.endpoint)]);
    let current = Arc::new(
        crate::tile_surface_wire::State::parse(
            &serde_json::to_vec(&v).unwrap(),
            &f.state.view.binding,
            "art",
            Some(&f.state),
        )
        .unwrap(),
    );
    let prompt = current.activity.confirmations[0].clone();
    let mut s = f.shared.lock();
    s.actions
        .observe(current, "origin", "device", Instant::now());
    s.actions.attempted(&c, &ack, Instant::now());
    prompt
}
#[test]
fn explicit_approve_reject_and_dialog_cancel_are_distinct_and_modal_reentry_revalidates() {
    let _com = Apartment::new();
    let f = Fixture::new("art");
    let w = window(&f);
    waiting(&f);
    assert!(w.test_art_choice(None));
    assert!(!queued(&f));
    for (choice, approved) in [(Choice::Approve, true), (Choice::Reject, false)] {
        waiting(&f);
        assert!(w.test_art_choice(Some(choice)));
        let command = f
            .shared
            .lock()
            .actions
            .take(&f.state, Instant::now())
            .unwrap();
        assert!(matches!(command.kind,Kind::Decision{approved:a,..} if a==approved));
    }
    waiting(&f);
    let command = choices(&w.state).pop().unwrap();
    choose(w.hwnd, &w.state, command, |_, _| {
        f.shared.stop();
        Ok(Some(Choice::Approve))
    });
    assert!(!queued(&f));
}
#[test]
fn suspension_keeps_confirmation_identity_but_requires_visible_resume_and_explicit_reject() {
    let _com = Apartment::new();
    for black in [false, true] {
        let f = Fixture::new("art");
        let w = window(&f);
        let prompt = waiting(&f);
        let current = f.shared.lock().actions.states["art"].clone();
        {
            let mut s = f.shared.lock();
            s.frozen = !black;
            s.black = black;
            s.actions.suspend();
        }
        assert!(!w.test_art_choice(Some(Choice::Reject)));
        assert!(!queued(&f));
        {
            let mut s = f.shared.lock();
            s.frozen = false;
            s.black = false;
            s.actions
                .observe(current.clone(), "origin", "device", Instant::now());
        }
        w.tick().unwrap();
        assert!(w.test_art_choice(Some(Choice::Reject)));
        let mut command = f
            .shared
            .lock()
            .actions
            .take(&current, Instant::now())
            .unwrap();
        assert!(
            matches!(&command.kind, Kind::Decision { confirmation, approved: false } if confirmation.as_ref() == &prompt)
        );
        assert!(crate::tile_surface_action_dispatch::validate(
            &mut command,
            &current,
            Some(&f.art),
            &f.job,
            &f.shared
        ));
        f.shared.stop();
        assert!(choices(&w.state).is_empty());
        assert!(f.shared.lock().actions.accepted.is_empty());
    }
}
#[test]
fn changed_prompt_during_modal_cannot_reuse_old_approval() {
    let _com = Apartment::new();
    let f = Fixture::new("art");
    let w = window(&f);
    let c = waiting(&f);
    let command = choices(&w.state).pop().unwrap();
    choose(w.hwnd, &w.state, command, |_, _| {
        let mut v = value(&f.job.key, "art");
        let mut next = confirmation(&c.event_id, &f.job.key.endpoint);
        next["payload"] = serde_json::json!({"different":true});
        v["confirmations"] = serde_json::json!([next]);
        let current = Arc::new(
            crate::tile_surface_wire::State::parse(
                &serde_json::to_vec(&v).unwrap(),
                &f.state.view.binding,
                "art",
                Some(&f.state),
            )
            .unwrap(),
        );
        f.shared
            .lock()
            .actions
            .observe(current, "origin", "device", Instant::now());
        Ok(Some(Choice::Approve))
    });
    assert!(!queued(&f));
}
