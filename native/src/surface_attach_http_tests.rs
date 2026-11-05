use super::*;
use crate::{
    live_relay_http::Persistence,
    surface_host_capabilities::{Input, Report, Runtime},
};
#[test]
fn attach_sends_explicit_capabilities_and_checks_returned_art_identity() {
    for mismatch in 0..4 {
        let (listener, client, config) = fixture();
        let session = session(&client.origin, u64::MAX);
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/surfaces/attach "));
            assert_eq!(body["artId"], "art:1");
            assert_eq!(body["hookNodeId"], "node:1");
            assert_eq!(body["deviceId"], "device-1");
            assert_eq!(body["persistence"], "persistent");
            assert_eq!(
                body["capabilities"]["runtimes"],
                serde_json::json!(["declarative"])
            );
            assert_eq!(body["capabilities"]["input"]["keyboard"], false);
            let mut response = mounted();
            response["instance"]["descriptor"]["artId"] =
                if mismatch == 2 { "other" } else { "art:1" }.into();
            response["instance"]["attachments"]["attachment:1"]["snapshot"]["runtime"] =
                if mismatch == 3 {
                    "javascript"
                } else {
                    "declarative"
                }
                .into();
            response["instance"]["attachments"]["attachment:1"]["snapshot"]["artId"] =
                if mismatch == 1 { "other" } else { "art:1" }.into();
            reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
        });
        let cancel = Cancel::new();
        let report = Report::new(
            vec![Runtime::Declarative],
            vec!["text".into()],
            vec![],
            vec![],
            Input::default(),
        )
        .unwrap();
        let result = Control::new(&client, &session, &cancel).attach_surface(
            "art:1",
            "node:1",
            &report,
            Persistence::Persistent,
        );
        if mismatch != 0 {
            assert!(matches!(result, Err(LiveError::SurfaceBinding(_))));
        } else {
            assert_eq!(result.unwrap().binding.attachment(), "attachment:1");
        }
        peer.join().unwrap();
    }
}
