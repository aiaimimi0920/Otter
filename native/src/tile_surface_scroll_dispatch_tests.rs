//! 滚动命令通过真实唯一worker/TLS；接收端独立核节点/负载/事件ID与恰一次POST。
use super::*;
#[test]
fn scrolled_button_is_validated_in_saved_geometry_and_posts_exactly_once() {
    let (listener, client, config, _) = crate::device_session_client::tests::fixture_with_pem();
    let mut grant = crate::device_authorization_lease::Grant::new(
        crate::device_session_client::tests::session(client.origin(), u64::MAX),
    );
    let lease = grant.lease(client.origin(), 0).unwrap();
    let f = crate::tile_surface_value::tests::fixture(
        serde_json::json!({"id":"root","type":"scroll","layout":{"width":64,"height":64},"children":[
        {"id":"gap","type":"view","layout":{"height":100,"minHeight":100}},
        {"id":"send","type":"button","layout":{"height":20,"minHeight":20},"props":{"eventPayload":{"independent":17}},"events":{"click":"go"}}]}),
    );
    let mut p = crate::tile_surface_value::tests::presentation(&f);
    assert!(p.wheel([10., 10.], 1, 1000., false).unwrap());
    let commands =
        crate::tile_surface_click::prepare(&f.frame, &f.art, &p, [0, 1], Default::default())
            .unwrap();
    assert_eq!(commands.len(), 1);
    let Kind::Event(event) = &commands[0].kind else {
        panic!()
    };
    let id = event["eventId"].clone();
    assert!(f.shared.lock().actions.offer(commands, Instant::now()));
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/walls/surfaces/event HTTP/1.1"));
        assert_eq!(body["event"]["nodeId"], "send");
        assert_eq!(body["event"]["eventId"], id);
        assert_eq!(
            body["event"]["payload"],
            serde_json::json!({"independent":17})
        );
        reply(
            &mut stream,
            202,
            &serde_json::to_vec(&crate::tile_surface_actions::tests::ack_value(
                id.as_str().unwrap(),
                "succeeded",
            ))
            .unwrap(),
        );
        assert!(listener.accept().is_err());
    });
    let views = Views::new(&client, &lease);
    views.dispatch_state(
        f.state.clone(),
        Some(f.art.clone()),
        &f.shared,
        &f.job,
        false,
    );
    assert!(f.shared.lock().actions.pending.is_empty());
    assert_eq!(f.shared.lock().actions.notice, "Art操作已受理");
    views.dispatch_state(
        f.state.clone(),
        Some(f.art.clone()),
        &f.shared,
        &f.job,
        false,
    );
    peer.join().unwrap();
}
