//! Loopback TLS peer for the real form attach/action/snapshot transaction.
use super::*;

fn scene(kind: &str, value: serde_json::Value) -> serde_json::Value {
    let event = if matches!(kind, "switch" | "select") {
        "change"
    } else {
        "input"
    };
    serde_json::json!({"id":"field","type":kind,
        "layout":{"width":180,"height":40},
        "props":{"value":value,"min":0,"max":100,"step":1,
            "options":[{"value":"a","label":"First"},{"value":"b","label":"Second"}]},
        "events":{event:"edit"}})
}
pub(super) fn initial(kind: &str) -> serde_json::Value {
    scene(
        kind,
        match kind {
            "switch" => false.into(),
            "select" => "a".into(),
            "slider" | "number" => 50.into(),
            _ => "source".into(),
        },
    )
}
pub(super) fn expected(kind: &str) -> serde_json::Value {
    match kind {
        "switch" => true.into(),
        "select" => "b".into(),
        "slider" => 51.0.into(),
        "number" => 42.0.into(),
        _ => "中文🙂".into(),
    }
}
pub(super) fn serve(
    kind: &'static str,
    listener: std::net::TcpListener,
    config: Arc<rustls::ServerConfig>,
    report: serde_json::Value,
) {
    for stage in 0..3 {
        let mut stream = StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut stream);
        let response = match stage {
            0 => {
                assert!(head.starts_with("POST /v1/surfaces/attach "));
                assert_eq!(body["capabilities"], report);
                assert!(body["capabilities"]["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|node| node == kind));
                let mut response = mounted();
                response["instance"]["descriptor"]["artId"] = "art:1".into();
                let snapshot = &mut response["instance"]["attachments"]["attachment:1"]["snapshot"];
                snapshot["artId"] = "art:1".into();
                snapshot["runtime"] = "declarative".into();
                snapshot["scene"] = initial(kind);
                response
            }
            1 => {
                assert!(head.starts_with("POST /v1/surfaces/instances/surface:test%2Fpart/events "));
                assert_eq!(body["nodeId"], "field");
                assert_eq!(body["baseRevision"], 3);
                let commit = matches!(kind, "switch" | "select");
                assert_eq!(body["event"], if commit { "change" } else { "input" });
                assert_eq!(body["class"], if commit { "commit" } else { "continuous" });
                assert_eq!(body["action"], "edit");
                assert_eq!(body["payload"]["value"], expected(kind), "{kind}");
                serde_json::json!({"protocolVersion":"loom.surface.v1","instanceId":body["instanceId"],
                    "eventId":body["eventId"],"requestId":"request:form","accepted":true,"status":"succeeded"})
            }
            _ => {
                assert!(head.starts_with("GET /v1/surfaces/stream?"));
                serde_json::json!({"protocolVersion":"loom.surface-stream.v1","next":1,"reset":true,
                "messages":[{"method":"loom.surface.snapshot","params":{"generation":2,"snapshot":{
                    "protocolVersion":"loom.surface.v1","instanceId":"surface:test/part",
                        "attachmentId":"attachment:1","revision":4,
                        "scene":{"id":"accepted","type":"text","props":{"text":"Accepted"}}
                }}}]})
            }
        };
        reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
    }
}
