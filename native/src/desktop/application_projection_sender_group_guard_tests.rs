//! 组菜单身份、busy与有界结果；不准备网络，不通过同字节目录绕过选择身份。
use super::*;
#[test]
fn group_menu_has_real_command_and_stale_directory_never_prepares_config() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, _, _) = Files::new();
    unsafe {
        let (_class, mut app) = application(files.root());
        let origin = "https://loom.example";
        let unit = app.documents[0].state.unit_id.clone();
        let selected = directory(origin, 2);
        {
            let mut view = app.authorization.borrow_mut();
            view.origin = Some(origin.into());
            view.targets.directory = Some(selected.clone());
            view.sender.ready = true;
        }
        let command = ui::sender::groups::BASE;
        assert!(crate::desktop::application::handles(command));
        assert!(crate::desktop::group_lock::allows(command));
        let menu = CreatePopupMenu().unwrap();
        ui::append_menu(menu, &app.documents[0].state).unwrap();
        assert_eq!(
            GetMenuState(menu, command as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        DestroyMenu(menu).unwrap();
        ui::execute(app.documents[0].window.0, &app.documents[0].state, command).unwrap();
        let request = app.authorization.borrow_mut().request.take().unwrap();
        let ui::Request::SendProjectionGroup(selection) = request else {
            panic!("expected group selection")
        };
        assert_eq!(selection.unit, unit);
        assert!(std::sync::Arc::ptr_eq(&selection.directory, &selected));
        app.authorization.borrow_mut().targets.directory = Some(directory(origin, 2));
        app.start_projection_group(selection, &mut |_| {
            panic!("stale group cannot prepare config")
        });
        assert!(app.projection_sender.group.is_none());
        assert!(!app.remote_sessions.projection_pending());
        for mode in 0..5 {
            let mut view = app.authorization.borrow_mut();
            view.sender.ready = true;
            view.sender.busy = mode == 0;
            view.busy = mode == 1;
            view.targets.busy = mode == 2;
            view.closed = mode == 3;
            view.origin = Some(
                if mode == 4 {
                    "https://changed.example"
                } else {
                    origin
                }
                .into(),
            );
            assert!(!ui::sender::groups::enabled(&view, &unit, 0));
        }
        app.shutdown();
    }
}
#[test]
fn offline_only_group_stays_disabled_and_no_online_route_is_inferred() {
    let mut view = ui::View::default();
    view.sender.ready = true;
    view.origin = Some("https://loom.example".into());
    let mut value = crate::projection_delivery::targets::tests::body();
    value["targets"] = json!([]);
    value["groups"][0]["targetIds"] = json!([]);
    value["groups"][0]["unavailableCount"] = json!(64);
    view.targets.directory = Some(
        crate::projection_delivery::targets::Directory::parse(
            &serde_json::to_vec(&value).unwrap(),
            "https://loom.example",
        )
        .unwrap()
        .into(),
    );
    assert!(!ui::sender::groups::enabled(
        &view,
        &crate::workspace_id::UnitId::parse("unit-1").unwrap(),
        0
    ));
    assert!(!ui::sender::handles(
        ui::sender::groups::STOP_BASE + crate::projection_delivery::targets::MAX_GROUPS
    ));
}
