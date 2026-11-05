//! 实际Viewer窗口在原decoder/D2D上传后的帧与独立颜色oracle；不注入用户输入。
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
    for (name, codec) in [
        ("viewer-h264", Codec::H264),
        ("viewer-raw", Codec::RawBgra),
        ("viewer-recovered", Codec::H264),
        ("viewer-fallback", Codec::RawBgra),
    ] {
        // 绘制层按原合同只持RawBgra；wire codec由真实服务/WS独立确认，并以精确帧号关联。
        context.signal(app, &format!("wire-{name}.json"));
        let wire: serde_json::Value = serde_json::from_slice(
            &std::fs::read(context.root.join(format!("wire-{name}.json"))).unwrap(),
        )
        .unwrap();
        let frame_id = wire["frameId"].as_u64().unwrap();
        assert_eq!(
            wire["codec"],
            if codec == Codec::H264 {
                "h264"
            } else {
                "raw_bgra"
            }
        );
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
        if codec == Codec::RawBgra {
            assert_eq!(color, [17, 83, 211, 255]);
        }
        assert_eq!(metadata.codec, Codec::RawBgra);
        context.record(
            &format!("{name}.json"),
            serde_json::json!({"frameId":metadata.frame_id,
            "codec":if codec==Codec::H264 {"h264"} else {"raw_bgra"},"keyframe":metadata.keyframe,
            "pixel":color,"width":64,"height":64,"nativeDecoderAndUpload":true}),
        );
        if name == "viewer-h264" && std::env::var_os("OTTER_LIVE_CONTROL_RECOVERY_JOINT").is_some()
        {
            context.signal(app, "viewer-control-disconnect");
            let (hwnd, _) = app.remote_sessions.test_viewer_input(key).unwrap();
            context.spin(app, |app| {
                app.remote_sessions.test_disconnect_viewer_control(key)
            });
            context.spin(app, |app| {
                app.remote_sessions
                    .test_viewer_control(key)
                    .is_some_and(|(recovering, _)| recovering)
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
        if name == "viewer-h264" && std::env::var_os("OTTER_LIVE_MEDIA_RECOVERY_JOINT").is_some() {
            context.signal(app, "viewer-disconnect");
            let (hwnd, _) = app.remote_sessions.test_viewer_input(key).unwrap();
            assert!(app.remote_sessions.test_disconnect_viewer_media(key));
            context.spin(app, |app| {
                app.remote_sessions
                    .test_viewer_input(key)
                    .is_some_and(|(_, status)| status.recovering)
            });
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
                    && app.remote_sessions.test_viewer_input(key).is_some_and(
                        |(current, status)| {
                            current == hwnd && !status.recovering && !status.enabled
                        },
                    )
            });
            let (metadata, color) = app.remote_sessions.test_frame(key).unwrap().unwrap();
            pixel(color);
            assert!(metadata.keyframe);
            assert_eq!(metadata.epoch, 1);
            context.record("viewer-media-resumed.json", serde_json::json!({"frameId":resumed,"pixel":color,
                "sameWindow":true,"epoch":1,"controlNotReacquired":true,"nativeDecoderAndUpload":true}));
        }
        if name == "viewer-h264"
            && std::env::var("OTTER_LIVE_SOURCE_RECOVERY_JOINT").as_deref() == Ok("1")
        {
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
    }
}
