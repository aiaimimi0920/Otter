//! Actual Viewer-window frames after the original decoder/D2D upload, checked against an independent color oracle; no user-input injection.
use super::*;
use std::sync::Arc;
pub(super) fn pixel(pixel: [u8; 4]) {
    for (actual, expected) in pixel.into_iter().zip([17u8, 83, 211, 255]) {
        assert!(
            actual.abs_diff(expected) <= 10,
            "independent color oracle: {pixel:?}"
        );
    }
}
pub(super) unsafe fn run(
    context: &context::Context,
    app: &mut Application,
    key: &Arc<crate::desktop::composition::relay::sessions::ViewerKey>,
) {
    use crate::live_relay_frame::Codec;
    let name = "viewer-h264";
    // The drawing layer retains only RawBgra under the original contract; real service/WS independently confirms wire codec, correlated by exact frame number.
    context.signal(app, &format!("wire-{name}.json"));
    let wire: serde_json::Value = serde_json::from_slice(
        &std::fs::read(context.root.join(format!("wire-{name}.json"))).unwrap(),
    )
    .unwrap();
    let frame_id = wire["frameId"].as_u64().unwrap();
    assert_eq!(wire["codec"], "h264");
    context.spin(app, |app| {
        app.remote_sessions
            .test_frame(key)
            .unwrap()
            .is_some_and(|(m, _)| m.frame_id == frame_id)
    });
    let (metadata, color) = app.remote_sessions.test_frame(key).unwrap().unwrap();
    pixel(color);
    assert!(
        metadata.keyframe,
        "each static policy transition must be independently decodable"
    );
    assert_eq!((metadata.width, metadata.height), (64, 64));
    assert_eq!(metadata.codec, Codec::RawBgra);
    context.record(
        &format!("{name}.json"),
        serde_json::json!({"frameId":metadata.frame_id,
            "codec":"h264","keyframe":metadata.keyframe,
            "pixel":color,"width":64,"height":64,"nativeDecoderAndUpload":true}),
    );
    // Both disconnect receipts must observe the original window and disabled state before recovery.
    let disconnected = |app: &Application, hwnd: HWND| {
        app.remote_sessions
            .test_viewer_input(key)
            .is_some_and(|(current, status)| {
                current == hwnd
                    && IsWindow(Some(current)).as_bool()
                    && status.recovering
                    && !status.enabled
            })
    };
    if std::env::var_os("OTTER_LIVE_CONTROL_RECOVERY_JOINT").is_some() {
        context.signal(app, "viewer-control-disconnect");
        let (hwnd, _) = app.remote_sessions.test_viewer_input(key).unwrap();
        context.spin(app, |app| {
            app.remote_sessions.test_disconnect_viewer_control(key)
        });
        context.spin(app, |app| {
            app.remote_sessions
                .test_viewer_control(key)
                .is_some_and(|(recovering, _)| recovering)
                && disconnected(app, hwnd)
        });
        let (_, after) = app.remote_sessions.test_viewer_control(key).unwrap();
        context.record(
            "viewer-control-disconnected.json",
            serde_json::json!({"after":after,"sameWindow":true,"recovering":true}),
        );
        context.signal(app, "wire-viewer-control-resumed.json");
        let wire: serde_json::Value = serde_json::from_slice(
            &std::fs::read(context.root.join("wire-viewer-control-resumed.json")).unwrap(),
        )
        .unwrap();
        let next = wire["after"].as_u64().unwrap();
        assert!(next > after);
        context.spin(app, |app| {
            app.remote_sessions
                .test_viewer_control(key)
                .is_some_and(|(recovering, cursor)| !recovering && cursor == next)
        });
        let (current, status) = app.remote_sessions.test_viewer_input(key).unwrap();
        assert_eq!(current, hwnd);
        assert!(!status.enabled && !status.recovering);
        let (frame, color) = app.remote_sessions.test_frame(key).unwrap().unwrap();
        pixel(color);
        context.record("viewer-control-resumed.json",serde_json::json!({"after":next,"sameWindow":true,"epoch":frame.epoch,"frameId":frame.frame_id,"pixel":color,"controlNotReacquired":true}));
    }
    if std::env::var_os("OTTER_LIVE_MEDIA_RECOVERY_JOINT").is_some() {
        context.signal(app, "viewer-disconnect");
        let (hwnd, _) = app.remote_sessions.test_viewer_input(key).unwrap();
        assert!(app.remote_sessions.test_disconnect_viewer_media(key));
        context.spin(app, |app| disconnected(app, hwnd));
        context.record(
            "viewer-disconnected.json",
            serde_json::json!({"frameId":frame_id,
                "sameWindow":true,"recovering":true,"controlDisabled":true}),
        );
        context.signal(app, "wire-viewer-media-resumed.json");
        let wire: serde_json::Value = serde_json::from_slice(
            &std::fs::read(context.root.join("wire-viewer-media-resumed.json")).unwrap(),
        )
        .unwrap();
        let resumed = wire["frameId"].as_u64().unwrap();
        assert!(resumed > frame_id);
        context.spin(app, |app| {
            app.remote_sessions
                .test_frame(key)
                .unwrap()
                .is_some_and(|(metadata, _)| metadata.frame_id == resumed)
                && app
                    .remote_sessions
                    .test_viewer_input(key)
                    .is_some_and(|(current, status)| {
                        current == hwnd && !status.recovering && !status.enabled
                    })
        });
        let (metadata, color) = app.remote_sessions.test_frame(key).unwrap().unwrap();
        pixel(color);
        assert!(metadata.keyframe);
        assert_eq!(metadata.epoch, 1);
        context.record("viewer-media-resumed.json", serde_json::json!({"frameId":resumed,"pixel":color,
                "sameWindow":true,"epoch":1,"controlNotReacquired":true,"nativeDecoderAndUpload":true}));
    }
    if std::env::var("OTTER_LIVE_SOURCE_RECOVERY_JOINT").as_deref() == Ok("1") {
        let (hwnd, _) = app.remote_sessions.test_viewer_input(key).unwrap();
        context.signal(app, "wire-viewer-source-resumed.json");
        let wire: serde_json::Value = serde_json::from_slice(
            &std::fs::read(context.root.join("wire-viewer-source-resumed.json")).unwrap(),
        )
        .unwrap();
        let resumed = wire["frameId"].as_u64().unwrap();
        assert!(resumed > frame_id);
        context.spin(app, |app| {
            app.remote_sessions
                .test_frame(key)
                .unwrap()
                .is_some_and(|(m, _)| m.frame_id == resumed)
        });
        let (metadata, color) = app.remote_sessions.test_frame(key).unwrap().unwrap();
        pixel(color);
        assert!(metadata.keyframe && metadata.epoch == 1);
        let (current, status) = app.remote_sessions.test_viewer_input(key).unwrap();
        assert_eq!(current, hwnd);
        assert!(!status.enabled);
        context.record("viewer-source-resumed.json", serde_json::json!({"frameId":resumed,"pixel":color,"sameWindow":true,"epoch":1,"controlNotReacquired":true,"nativeDecoderAndUpload":true}));
    }
    if std::env::var_os("OTTER_LIVE_WGC_JOINT").is_some() {
        context.signal(app, "wire-viewer-wgc-updated.json");
        let wire: serde_json::Value = serde_json::from_slice(
            &std::fs::read(context.root.join("wire-viewer-wgc-updated.json")).unwrap(),
        )
        .unwrap();
        let next = wire["frameId"].as_u64().unwrap();
        assert!(next > frame_id);
        context.spin(app, |app| {
            app.remote_sessions
                .test_frame(key)
                .unwrap()
                .is_some_and(|(m, color)| {
                    m.frame_id >= next
                        && m.width == 96
                        && m.height == 80
                        && color
                            .into_iter()
                            .zip([193u8, 29, 71, 255])
                            .all(|(a, b)| a.abs_diff(b) <= 10)
                })
        });
        let (metadata, color) = app.remote_sessions.test_frame(key).unwrap().unwrap();
        context.record("viewer-wgc-updated.json", serde_json::json!({"frameId":metadata.frame_id,
            "width":metadata.width,"height":metadata.height,"pixel":color,"nativeDecoderAndUpload":true}));
    }
}
