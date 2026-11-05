//! 原receiver read后才自动申请原Apply；停止在真实worker已完成边界切换有界TLS夹具。
use super::*;
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, GetMenuState, MF_BYCOMMAND, MF_CHECKED, MF_GRAYED,
};

unsafe fn modes(fixture: &Fixture, two_way: bool) {
    let menu = CreatePopupMenu().unwrap();
    let state = state(fixture);
    crate::desktop::device_authorization_ui::append_menu(menu, state).unwrap();
    for (id, checked) in [
        (
            crate::desktop::device_authorization_ui::editing::ONE_WAY,
            !two_way,
        ),
        (
            crate::desktop::device_authorization_ui::editing::TWO_WAY,
            two_way,
        ),
    ] {
        let flags = GetMenuState(menu, id as u32, MF_BYCOMMAND);
        assert_ne!(flags, u32::MAX);
        assert_ne!(flags & MF_GRAYED.0, 0, "receiver cannot change mode");
        assert_eq!(flags & MF_CHECKED.0 != 0, checked);
    }
    DestroyMenu(menu).unwrap();
}
unsafe fn received(offline: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = accepted_two_way(offline);
        modes(&fixture, true);
        shift(&fixture, 0.25);
        let mut response = wire(&fixture.origin, 2, offline);
        response["snapshot"] = Value::Null;
        response["editing"]["mode"] = json!("two_way");
        let rx = peer(&mut fixture, vec![response]);
        completed(&mut fixture, &rx);
        // 不sleep猜时序：尚未poll领取read，先join恢复listener，安装原Apply夹具。
        joined(&mut fixture);
        let rx = server(&mut fixture, vec![(200, 0.5)]);
        poll(&mut fixture);
        assert!(!fixture.app.projection_receive.busy());
        assert!(
            fixture.app.projection_edit.busy(),
            "successful read must trigger automatic apply"
        );
        edit_completed(&fixture);
        let request = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(request["baseRevision"], 2);
        assert_eq!(request["modeRevision"], 1);
        poll_edit(&mut fixture);
        assert!(journal(&fixture)["entries"][0]["pending"].is_object());
        finish(&mut fixture);
        joined(&mut fixture);
        assert!(journal(&fixture)["entries"][0]["pending"].is_null());
        assert_eq!(
            snapshot(&fixture).document().objects()["remote"].value()["x"].as_f64(),
            Some(0.5)
        );
        super::super::super::super::fixture::reopen_workspace(&mut fixture);
        assert_eq!(snapshot(&fixture).document().revision(), 3);
        assert_eq!(
            snapshot(&fixture).editor_view().unwrap().basis_objects()["remote"]["x"].as_f64(),
            Some(0.5)
        );
    }
}
#[test]
fn receiver_shared_successful_read_automatically_submits_then_checkpoint_and_reopens() {
    unsafe { received(false) }
}
#[test]
fn receiver_offline_successful_read_automatically_submits_then_checkpoint_and_reopens() {
    unsafe { received(true) }
}
#[test]
fn receiver_one_way_or_real_object_conflict_never_automatically_write() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        for two_way in [false, true] {
            let mut fixture = if two_way {
                accepted_two_way(false)
            } else {
                accepted(false)
            };
            modes(&fixture, two_way);
            shift(&fixture, 0.25);
            let mut response = wire(&fixture.origin, 2, false);
            response["snapshot"] = Value::Null;
            if two_way {
                response["editing"]["mode"] = json!("two_way");
                response["editing"]["objects"]["remote"]["value"]["x"] = json!(0.25);
            }
            let rx = peer(&mut fixture, vec![response]);
            cycle(&mut fixture, &rx);
            joined(&mut fixture);
            assert!(!fixture.app.projection_edit.busy());
            assert!(snapshot(&fixture).conflict().is_some());
            assert!(!fixture.files.root().join("projection-edit.json").exists());
            assert_eq!(
                snapshot(&fixture).editor_view().unwrap().basis_objects()["remote"]["x"].as_f64(),
                Some(0.5)
            );
        }
    }
}
