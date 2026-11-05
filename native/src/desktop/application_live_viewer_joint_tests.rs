//! 双原生Application/真实Surface公共挂载/真实LAN HTTPS/WSS控制联合；非普通双EXE或物理跨机。
use super::*;
use crate::desktop::composition::relay::window::input::{ACQUIRE, RELEASE};
#[path = "application_live_viewer_joint_fixture.rs"]
mod context;
#[path = "application_local_art_trigger_joint.rs"]
mod local_trigger;
#[path = "application_live_viewer_trigger_joint.rs"]
mod trigger;
#[path = "application_live_viewer_video_joint.rs"]
mod video;
#[test]
#[ignore = "explicit native viewer menu to real Loom and original native Source input worker"]
fn viewer_menu_real_loom_source_input_and_release() {
    run(false);
}
#[test]
#[ignore = "explicit native viewer trigger configuration over real Loom LAN and actual Surface executor"]
fn viewer_trigger_real_loom_surface_action() {
    run(true);
}
fn run(triggers: bool) {
    run_mode(triggers, false);
}
#[test]
#[ignore = "explicit native H264 viewer and Source over real Loom LAN"]
fn viewer_h264_real_loom_source_policy_and_pixels() {
    run_mode(false, true);
}
fn run_mode(triggers: bool, video: bool) {
    let context = context::Context::load();
    let _com = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = fixture::application();
        context.authorize(&mut app);
        super::super::super::super::super::surface_start::start_with(
            &context.origin,
            context.data["artId"].as_str().unwrap().into(),
            app.documents[0].state.unit_id.to_string(),
            &mut app.authorization_owner,
            &mut app.remote_sessions,
            &app.surface_windows,
            |origin| context.config(origin),
        )
        .unwrap();
        context.spin(&mut app, |app| {
            app.surface_windows.iter_mut().any(|window| {
                window
                    .lifecycle()
                    .is_some_and(|owner| owner.confirmed_active())
            })
        });
        let now = crate::desktop::surface_windows::now_ms().unwrap();
        let (instance, attachment) = {
            let window = app.surface_windows.iter_mut().next().unwrap();
            context::Context::offscreen(window.edit_test_parent());
            window.show(now).unwrap();
            (
                window.binding().instance().to_owned(),
                window.binding().attachment().to_owned(),
            )
        };
        app.settle_live_viewers().unwrap();
        app.authorization.borrow_mut().request = Some(Request::Discover);
        app.process_authorization_with_projection(|origin| context.config(origin));
        context.spin(&mut app, |app| app.authorization.borrow().catalog.is_some());
        let selected = {
            let view = app.authorization.borrow();
            let entry = view
                .catalog
                .as_ref()
                .unwrap()
                .entries
                .iter()
                .find(|entry| entry.identity.session() == context.data["session"].as_str().unwrap())
                .unwrap();
            live::Selection {
                origin: context.origin.clone(),
                identity: entry.identity.clone(),
                binding: view.live.bindings[0].clone(),
            }
        };
        live::select(app.documents[0].window.0, &app.documents[0].state, selected).unwrap();
        app.process_authorization_with_projection(|origin| context.config(origin));
        context.spin(&mut app, |app| {
            app.remote_sessions
                .viewers()
                .first()
                .is_some_and(|viewer| viewer.shown)
        });
        let key = app.remote_sessions.viewers()[0].key.clone();
        let stamp = app.documents[0]
            .state
            .canvas
            .borrow()
            .editor
            .document_stamp();
        context.spin(&mut app, |app| {
            app.remote_sessions.test_frame(&key).unwrap().is_some()
        });
        let (metadata, pixel) = app.remote_sessions.test_frame(&key).unwrap().unwrap();
        assert_eq!(
            (metadata.width, metadata.height),
            if video { (64, 64) } else { (2, 2) }
        );
        if video {
            video::pixel(pixel);
        } else {
            assert_eq!(pixel, [17, 83, 211, 255]);
        }
        let (hwnd, status) = app.remote_sessions.test_viewer_input(&key).unwrap();
        context::Context::offscreen(hwnd);
        assert!(!status.enabled);
        // 观察发布不授键鼠控制；旧输入联合仍必须具备显式输入能力。
        assert_eq!(status.available, !triggers && !video);
        context.record("viewer-ready.json", serde_json::json!({"instance":instance,"attachment":attachment,"realSurface":true,"menu":true,"application":true,"pixel":pixel}));
        if video {
            video::run(&context, &mut app, &key);
        } else if triggers {
            trigger::run(&context, &mut app, hwnd, &instance, &attachment);
        } else {
            for (index, code) in [65, 66].into_iter().enumerate() {
                if index == 1 {
                    context.signal(&mut app, "viewer-phase-2");
                }
                SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(ACQUIRE)), None);
                context.spin(&mut app, |app| {
                    app.remote_sessions
                        .test_viewer_input(&key)
                        .is_some_and(|(_, s)| s.enabled)
                });
                SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(code)), None);
                let mut rect = RECT::default();
                GetClientRect(hwnd, &mut rect).unwrap();
                let point = LPARAM(
                    (((rect.bottom / 2) as usize) << 16 | (rect.right / 2) as usize) as isize,
                );
                SendMessageW(hwnd, WM_LBUTTONDOWN, Some(WPARAM(0)), Some(point));
                context.record(
                    &format!("viewer-sent-{}.json", index + 1),
                    serde_json::json!({"key":code,"mouseDown":true}),
                );
                context.signal(&mut app, &format!("viewer-release-{}", index + 1));
                if index == 0 {
                    SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(RELEASE)), None);
                } else {
                    SendMessageW(hwnd, WM_KILLFOCUS, Some(WPARAM(0)), None);
                }
                context.spin(&mut app, |app| {
                    app.remote_sessions
                        .test_viewer_input(&key)
                        .is_some_and(|(_, s)| !s.enabled && !s.pending)
                });
                assert!(IsWindow(Some(hwnd)).as_bool());
                assert_eq!(
                    app.remote_sessions.test_frame(&key).unwrap().unwrap().1,
                    [17, 83, 211, 255]
                );
                context.record(&format!("viewer-released-{}.json", index + 1), serde_json::json!({"stillViewing":true,"release":if index==0 {"menu"}else{"focus_loss"}}));
            }
        }
        context.signal(&mut app, "viewer-stop");
        app.authorization.borrow_mut().request = Some(Request::StopLive(key.clone()));
        app.process_authorization_with_projection(|origin| context.config(origin));
        context.spin(&mut app, |app| app.remote_sessions.viewers().is_empty());
        assert!(stamp.matches(&app.documents[0].state.canvas.borrow().editor));
        app.surface_windows.close_all().unwrap();
        context.spin(&mut app, |app| {
            app.surface_windows.iter_mut().all(|window| {
                window.lifecycle().is_some_and(|owner| {
                    owner.binding().lifecycle() == crate::surface_live_binding::Lifecycle::Inactive
                })
            })
        });
        assert_ne!(GetForegroundWindow(), hwnd);
        app.shutdown();
        assert!(app.remote_sessions.finished());
        context.record("viewer-complete.json", serde_json::json!({"closed":true,"ownersReclaimed":true,"editorUnchanged":true,"realSurfaceInactive":true,"ordinaryDualProduct":false,"physicalCrossDevice":false}));
    }
}
