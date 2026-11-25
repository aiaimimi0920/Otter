//! New left-side entries consume real authorization catalogs/original requests; no identity-credential reads, networking, or user-desktop clicks.
use super::*;
use crate::desktop::device_authorization_ui::{self as auth, settings as menus, Request};
use menus::Section;
fn find(items: &[crate::native_menu::Item], command: usize) -> Option<&crate::native_menu::Item> {
    items.iter().find_map(|item| {
        if item.command == command {
            Some(item)
        } else {
            item.children
                .as_ref()
                .and_then(|items| find(items, command))
        }
    })
}
#[test]
fn settings_show_connection_projection_receive_and_flow_controls_without_starting_requests() {
    unsafe {
        let f = Fixture::new();
        let panel = open(&f);
        for (id, expected) in [
            (201, "pin-connect"),
            (202, "pin-send"),
            (203, "pin-receive"),
            (204, "pin-flow"),
        ] {
            let child = field(panel, id);
            assert!(IsWindowVisible(child).as_bool());
            let mut text = [0; 64];
            let n = GetWindowTextW(child, &mut text) as usize;
            assert_eq!(
                String::from_utf16_lossy(&text[..n]),
                crate::ui_text::text(expected)
            );
        }
        let connection = menus::items(Section::Loom, &f.state).unwrap();
        assert!(find(&connection, auth::SELECT).unwrap().enabled);
        assert!(!find(&connection, auth::PAIR).unwrap().enabled);
        for command in [
            auth::OPEN_ART,
            auth::DISCOVER,
            auth::CONFIRM_SURFACE,
            auth::RESUME_SURFACE,
        ] {
            assert!(find(&connection, command).is_none());
        }
        let send = menus::items(Section::Devices, &f.state).unwrap();
        assert!(!find(&send, auth::targets::REFRESH).unwrap().enabled);
        let receive = menus::items(Section::Receive, &f.state).unwrap();
        for id in [
            auth::policy::AUTO,
            auth::policy::CONFIRM,
            auth::policy::REJECT,
            auth::policy::PAUSE,
        ] {
            assert!(find(&receive, id).unwrap().enabled);
        }
        let flow = menus::items(Section::Flow, &f.state).unwrap();
        assert_eq!(flow.len(), 1);
        assert!(!flow[0].enabled);
        assert!(f.state.authorization.borrow().request.is_none());
    }
}

#[test]
fn actual_sidebar_buttons_notify_and_owned_cascade_routes_policy_after_cleanup() {
    unsafe {
        let f = Fixture::new();
        let panel = open(&f);
        assert_eq!(
            GetAncestor(panel, GA_ROOTOWNER),
            GetAncestor(f.owner.0, GA_ROOTOWNER)
        );
        // Private desktops cannot become system foreground; verify actual button notifications and the same menu selector,
        // without claiming ordinary-desktop acceptance for foreground mouse input/nested message pumps.
        for id in [201, 202, 203] {
            let child = field(panel, id);
            let _ = SetFocus(Some(child));
            SendMessageW(child, BM_CLICK, None, None);
            let mut found = false;
            for _ in 0..32 {
                let mut message = MSG::default();
                if !PeekMessageW(
                    &mut message,
                    Some(f.owner.0),
                    tool_panel::EXECUTE,
                    tool_panel::EXECUTE,
                    PM_REMOVE,
                )
                .as_bool()
                {
                    break;
                }
                if message.lParam.0 == neuro_mini_ui::PANEL && message.wParam.0 == id as usize {
                    found = true;
                    break;
                }
            }
            assert!(found, "missing sidebar notification {id}");
        }
        menus::choose_with(f.owner.0, &f.state, Section::Receive, |items| {
            assert!(f.state.file_dialog.get());
            let steps = items
                .iter()
                .filter(|item| item.enabled && !item.separator)
                .position(|item| item.command == auth::policy::REJECT)
                .unwrap()
                + 1;
            let mut menu =
                crate::native_menu::cascade::Cascade::open(panel, items, POINT { x: 100, y: 100 })?;
            for _ in 0..steps {
                menu.key(VK_DOWN.0 as u32)?;
            }
            menu.key(VK_RETURN.0 as u32)?;
            assert!(menu.closed());
            Ok(menu.take_command())
        })
        .unwrap();
        assert!(matches!(
            f.state.authorization.borrow_mut().request.take(),
            Some(Request::ReceivePolicy(
                crate::projection_receive_policy::Policy::Reject
            ))
        ));
        assert!(f.state.error.borrow().is_none());
        assert!(!f.state.file_dialog.get());
        assert!(GetCapture().is_invalid());
        assert!(settings::is_open(&f.state));
    }
}

#[test]
fn same_named_directory_and_invitation_replacements_retire_open_menu_commands() {
    use crate::projection_delivery::{targets::Directory, Inbox};
    const ORIGIN: &str = "https://loom.example";
    let directory = || {
        std::sync::Arc::new(
            Directory::parse(
                &serde_json::to_vec(&crate::projection_delivery::targets::tests::body()).unwrap(),
                ORIGIN,
            )
            .unwrap(),
        )
    };
    let invitation = || {
        std::sync::Arc::new(Inbox::parse(
        &serde_json::to_vec(&serde_json::json!({"invitations":[crate::projection_delivery::tests::invitation(ORIGIN, false)]})).unwrap(),
        ORIGIN).unwrap().into_entries().pop().unwrap())
    };
    unsafe {
        let f = Fixture::new();
        open(&f);
        f.state.authorization.borrow_mut().origin = Some(ORIGIN.into());
        f.state.authorization.borrow_mut().targets.directory = Some(directory());
        menus::choose_with(f.owner.0, &f.state, Section::Devices, |_| {
            f.state.authorization.borrow_mut().targets.directory = Some(directory());
            Ok(Some(auth::targets::REFRESH))
        })
        .unwrap();
        assert!(f.state.authorization.borrow().request.is_none());
        f.state.authorization.borrow_mut().projection.entries = vec![invitation()];
        menus::choose_with(f.owner.0, &f.state, Section::Receive, |_| {
            f.state.authorization.borrow_mut().projection.entries = vec![invitation()];
            Ok(Some(auth::projection::REJECT_BASE))
        })
        .unwrap();
        assert!(f.state.authorization.borrow().request.is_none());
        menus::choose_with(f.owner.0, &f.state, Section::Receive, |_| {
            Ok(Some(auth::projection::REJECT_BASE))
        })
        .unwrap();
        assert!(matches!(
            f.state.authorization.borrow_mut().request.take(),
            Some(Request::Projection(auth::projection::Request::Reject(_)))
        ));
    }
}

#[test]
fn baseline_connection_and_projection_svgs_are_supported_by_the_native_painter() {
    for svg in [
        include_str!("../../assets/edit-strip/SettingsLoom.svg"),
        include_str!("../../assets/edit-strip/SettingsSend.svg"),
        include_str!("../../assets/edit-strip/SettingsReceive.svg"),
        include_str!("../../assets/edit-strip/SettingsFlow.svg"),
    ] {
        neuro_mini_ui::Icon::svg(svg).validate().unwrap();
    }
}
#[test]
fn settings_menu_releases_modal_before_queue_and_rejects_changed_origin_or_hidden_panel() {
    unsafe {
        let f = Fixture::new();
        open(&f);
        f.state.authorization.borrow_mut().origin = Some("https://localhost:1234".into());
        menus::choose_with(f.owner.0, &f.state, Section::Loom, |_| {
            assert!(f.state.file_dialog.get());
            Ok(Some(auth::PAIR))
        })
        .unwrap();
        assert!(matches!(
            f.state.authorization.borrow_mut().request.take(),
            Some(Request::Start(
                crate::device_authorization_config::Action::RequestPairing
            ))
        ));
        assert!(!f.state.file_dialog.get());
        for hide in [false, true] {
            menus::choose_with(f.owner.0, &f.state, Section::Loom, |_| {
                if hide {
                    settings::close(&f.state);
                } else {
                    f.state.authorization.borrow_mut().origin =
                        Some("https://localhost:4321".into());
                }
                Ok(Some(auth::PAIR))
            })
            .unwrap();
            assert!(f.state.authorization.borrow().request.is_none());
        }
    }
}
#[test]
fn settings_cannot_execute_disabled_or_other_section_commands_and_policy_uses_original_owner() {
    unsafe {
        let f = Fixture::new();
        open(&f);
        for id in [auth::PAIR, auth::policy::AUTO] {
            menus::choose_with(f.owner.0, &f.state, Section::Loom, |_| Ok(Some(id))).unwrap();
            assert!(f.state.authorization.borrow().request.is_none());
        }
        menus::choose_with(f.owner.0, &f.state, Section::Receive, |_| {
            Ok(Some(auth::policy::REJECT))
        })
        .unwrap();
        assert!(matches!(
            f.state.authorization.borrow_mut().request.take(),
            Some(Request::ReceivePolicy(
                crate::projection_receive_policy::Policy::Reject
            ))
        ));
        assert!(!f.state.file_dialog.get());
    }
}
