//! Live graph 身份准入、宿主替换、冻结身份交接和终止清理；合成帧不捕获用户桌面。
use super::*;
use crate::{
    workspace::{Group, Link},
    workspace_id::UnitId,
};
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn connect(app: &mut Application, from: &UnitId, to: &UnitId) {
    app.workspace
        .add_link(Link {
            id: id("live-link"),
            from_unit: from.clone(),
            from_port: "output".into(),
            to_unit: to.clone(),
            to_port: "image".into(),
        })
        .unwrap();
}
#[test]
fn live_workspace_freeze_keeps_registered_identity_links_and_saved_group() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, unit) = live(&mut app, true);
        let host = app.documents[0].state.unit_id.clone();
        assert!(app.workspace.contains(&unit));
        connect(&mut app, &unit, &host);
        let group = id("live-group");
        app.workspace
            .upsert_group(Group {
                id: group.clone(),
                name: "Live".into(),
                hidden: false,
                locked: false,
            })
            .unwrap();
        app.workspace
            .assign_group(std::slice::from_ref(&unit), Some(&group))
            .unwrap();
        app.request_save_candidate().unwrap();
        let saved = super::save::prepared(&mut app).unwrap();
        let document = crate::session_document::decode(&saved.candidate.bytes).unwrap();
        assert_eq!(document.workspace.group_of(&unit).unwrap().id, group);
        assert_eq!(document.workspace.links(), app.workspace.links());
        let topology = app.workspace.topology_stamp();
        queue(hwnd);
        app.process_live_freeze(false).unwrap();
        settle(&mut app).unwrap();
        assert!(topology.matches(&app.workspace));
        assert_eq!(
            app.workspace
                .units()
                .iter()
                .filter(|id| **id == unit)
                .count(),
            1
        );
        assert_eq!(
            app.documents[1].state.group_id.borrow().as_ref(),
            Some(&group)
        );
        app.shutdown_live().unwrap();
        assert!(app.workspace.contains(&unit));
        assert_eq!(app.workspace.links().len(), 1);
    }
}
#[test]
fn live_workspace_last_static_recycle_preserves_live_units_and_unrelated_links() {
    unsafe {
        let environment = Environment::new();
        let image = crate::desktop::image::Image::from_encoded(
            crate::desktop::application::smoke::image_fixture::PNG,
        )
        .unwrap();
        let mut app = Application::new(
            environment.instance,
            DocumentWindow::create(
                environment.instance,
                State::new(Some(image), false).unwrap(),
                false,
            )
            .unwrap(),
        );
        let (_, first) = live(&mut app, true);
        let (_, second) = live(&mut app, true);
        connect(&mut app, &first, &second);
        let identity = app.workspace.identity_stamp();
        let host = &app.documents[0];
        let old = host.state.unit_id.clone();
        app.sync_frozen_menu();
        let host = &app.documents[0];
        crate::desktop::context_menu::execute(
            host.window.0,
            &host.state,
            crate::desktop::reference_ui::RECYCLE,
        )
        .unwrap();
        app.process_reference(false).unwrap();
        super::recycle::settle_recycle(&mut app).unwrap();
        assert!(!app.workspace.contains(&old));
        assert!(identity.matches(&app.workspace));
        assert!(app.workspace.contains(&first) && app.workspace.contains(&second));
        assert_eq!(app.workspace.links().len(), 1);
        assert_eq!(app.workspace.units().len(), 3);
        assert!(app.live_sessions.iter().all(Session::is_alive));
        app.shutdown_live().unwrap();
        assert_eq!(app.workspace.units().len(), 1);
        assert!(app.workspace.links().is_empty());
    }
}
#[test]
fn live_workspace_static_collection_and_discard_follow_actual_owner_lifetime() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        app.add_state(State::new(None, false).unwrap(), false)
            .unwrap();
        let (hwnd, unit) = live(&mut app, true);
        DestroyWindow(app.documents[0].window.0).unwrap();
        app.collect_closed();
        assert!(app.workspace.contains(&unit));
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1fc0)), Some(LPARAM(0)));
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.live_sessions.is_empty() {
            app.process_live(false).unwrap();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!app.workspace.contains(&unit));
        assert_eq!(app.workspace.units().len(), 1);
    }
}
#[test]
fn live_workspace_full_graph_rejects_and_reclaims_new_session_atomically() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        app.workspace
            .add_units(
                (0..crate::workspace::MAX_UNITS - 1).map(|index| id(&format!("full-{index}"))),
            )
            .unwrap();
        let topology = app.workspace.topology_stamp();
        let (hwnd, session) = live_session(true);
        let unit = session.unit_id().clone();
        assert!(app.admit_live(session, false).is_err());
        assert!(topology.matches(&app.workspace));
        assert!(!app.workspace.contains(&unit));
        assert!(app.live_sessions.is_empty());
        assert!(!IsWindow(Some(hwnd)).as_bool());
    }
}
