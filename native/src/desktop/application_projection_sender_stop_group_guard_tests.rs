//! 组停止固定canonical allocation与目录Arc；旧菜单不允许从当前原文重抓新依据。
use super::*;
fn stale_selection(source_changed: bool) {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let (files, identity, _) = Files::new();
    let (listener, client, tls, pem) = fixture_with_pem();
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("ca.pem"), pem).unwrap();
    let server = peer::start_stopping(listener, tls, files.root().to_owned(), 6, false);
    unsafe {
        let (_class, mut app) = application(files.root());
        authorize(&mut app, &files, identity, client);
        send_both(&mut app, &files, &origin);
        let original = std::fs::read(files.root().join("projection-sender.json")).unwrap();
        stop_menu(&app);
        let ui::Request::StopProjectionGroup(mut selected) =
            app.authorization.borrow_mut().request.take().unwrap()
        else {
            panic!("expected real group stop selection");
        };
        if source_changed {
            selected.source = selected.source.to_vec().into();
        } else {
            app.authorization.borrow_mut().targets.directory = Some(directory(&origin, 2));
        }
        app.start_projection_stop_group(selected);
        assert!(app.projection_sender.stop_group.is_none());
        assert!(!app.projection_sender.stop.pending());
        assert!(!app.remote_sessions.projection_pending());
        assert_eq!(
            std::fs::read(files.root().join("projection-sender.json")).unwrap(),
            original
        );
        let unit = app.documents[0].state.unit_id.clone();
        for mode in 0..5 {
            let mut view = app.authorization.borrow_mut();
            view.sender.can_stop = mode != 0;
            view.closed = mode == 1;
            view.targets.busy = mode == 2;
            view.origin = Some(
                if mode == 3 {
                    "https://other.example"
                } else {
                    &origin
                }
                .into(),
            );
            if mode == 4 {
                view.sender.stop_source = None;
            }
            assert!(!ui::sender::groups::stop_enabled(&view, &unit, 0));
        }
        app.shutdown();
    }
    assert_eq!(server.join().unwrap().len(), 2);
}
#[test]
fn group_stop_stale_canonical_allocation_is_rejected_even_with_equal_bytes() {
    stale_selection(true);
}
#[test]
fn group_stop_stale_directory_arc_is_rejected_and_disabled_states_do_not_queue() {
    stale_selection(false);
}
