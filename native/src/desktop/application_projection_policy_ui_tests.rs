//! 自有隐藏窗口的共享菜单→唯一 profile worker→磁盘提交；不注入全局输入。
use super::*;
use crate::{
    desktop::{
        device_authorization_ui::{self as ui, policy},
        workspace_profile::Profile,
    },
    projection_receive_policy::Policy,
};
use std::time::{Duration, Instant};
fn finish(app: &mut Application) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        app.process_authorization();
        if matches!(
            app.projection_profile,
            projection::profile::Owner::Ready { .. }
        ) && !app.authorization.borrow().consent.saving
        {
            break;
        }
        assert!(
            Instant::now() < end,
            "receive policy UI completion deadline"
        );
        std::thread::yield_now();
    }
}
#[test]
fn receive_policy_shared_native_menu_persists_and_survives_service_reset() {
    let _com = Apartment::new().unwrap();
    let root = missing_profile();
    std::fs::create_dir(&root).unwrap();
    unsafe {
        let (_class, mut app) = application();
        app.install_workspace_checkpoint(Some(Profile::open(&root).unwrap()), false)
            .unwrap();
        finish(&mut app);
        let second = app.bind_window_state(State::new(None, false).unwrap());
        for (command, value) in [
            (policy::AUTO, Policy::Auto),
            (policy::REJECT, Policy::Reject),
            (policy::CONFIRM, Policy::Confirm),
        ] {
            let before = app.authorization.borrow().consent.policy();
            ui::execute(app.documents[0].window.0, &app.documents[0].state, command).unwrap();
            assert_eq!(second.authorization.borrow().consent.policy(), before);
            app.process_projection_policy();
            assert!(second.authorization.borrow().consent.saving);
            assert_eq!(second.authorization.borrow().consent.policy(), before);
            finish(&mut app);
            assert_eq!(second.authorization.borrow().consent.policy(), value);
            let stored = crate::projection_receive_policy::Document::decode(
                &std::fs::read(root.join("projection-receive-policy.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(stored.policy(), value);
            let menu = CreatePopupMenu().unwrap();
            ui::append_menu(menu, &app.documents[0].state).unwrap();
            assert_ne!(
                GetMenuState(menu, command as u32, MF_BYCOMMAND) & MF_CHECKED.0,
                0
            );
            DestroyMenu(menu).unwrap();
            app.authorization.borrow_mut().request =
                Some(Request::Select("https://selected.example".into()));
            app.process_authorization_with_projection(|_| {
                panic!("policy selection started network")
            });
            app.authorization.borrow_mut().request = Some(Request::Cancel);
            app.process_authorization_with_projection(|_| panic!("reset started network"));
            assert_eq!(second.authorization.borrow().consent.policy(), value);
        }
        app.shutdown();
        assert!(!second.authorization.borrow().consent.saving);
    }
}
#[test]
fn receive_policy_without_native_profile_cannot_grant_auto_or_change_other_settings() {
    let _com = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = application();
        let source = app.naming_document.clone();
        ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            policy::AUTO,
        )
        .unwrap();
        app.process_authorization_with_projection(|_| panic!("policy save read network config"));
        assert_eq!(app.authorization.borrow().consent.policy(), Policy::Confirm);
        assert_eq!(
            app.authorization.borrow().consent.status,
            "保存接收策略失败"
        );
        assert!(std::sync::Arc::ptr_eq(&source, &app.naming_document));
        app.shutdown();
    }
}

#[test]
fn receive_policy_pause_menu_is_shared_without_service_and_blocks_inbox_not_consent_controls() {
    let _com = Apartment::new().unwrap();
    let root = missing_profile();
    std::fs::create_dir(&root).unwrap();
    unsafe {
        let (_class, mut app) = application();
        app.install_workspace_checkpoint(Some(Profile::open(&root).unwrap()), false)
            .unwrap();
        finish(&mut app);
        let second = app.bind_window_state(State::new(None, false).unwrap());
        for paused in [true, false] {
            ui::execute(
                app.documents[0].window.0,
                &app.documents[0].state,
                policy::PAUSE,
            )
            .unwrap();
            finish(&mut app);
            assert_eq!(second.authorization.borrow().consent.paused(), paused);
            assert_eq!(
                second.authorization.borrow().consent.policy(),
                Policy::Confirm
            );
            app.authorization.borrow_mut().origin = Some("https://loom.example".into());
            let menu = CreatePopupMenu().unwrap();
            ui::append_menu(menu, &app.documents[0].state).unwrap();
            assert_eq!(
                GetMenuState(menu, ui::projection::REFRESH as u32, MF_BYCOMMAND) & MF_GRAYED.0 != 0,
                paused
            );
            let mut text = [0u16; 64];
            let len = GetMenuStringW(menu, policy::PAUSE as u32, Some(&mut text), MF_BYCOMMAND);
            assert_eq!(
                String::from_utf16(&text[..len as usize]).unwrap(),
                if paused {
                    "恢复本机接收"
                } else {
                    "暂停本机接收"
                }
            );
            DestroyMenu(menu).unwrap();
            if paused {
                app.authorization.borrow_mut().request =
                    Some(Request::Projection(ui::projection::Request::Refresh));
                app.process_authorization_with_projection(|_| {
                    panic!("paused direct inbox reached config")
                });
                assert!(!app.remote_sessions.projection_pending());
            }
            app.authorization.borrow_mut().origin = None;
        }
        app.shutdown();
    }
}
