//! 合成认证挂载 + 真实续签/TLS GET + 完整应用 poll 安装；不手动 start/install。
use super::*;
use crate::{
    desktop::Apartment,
    device_session_client::tests::{accept, fixture_with_pem, pairing::Files, reply, request},
};
use rustls::{ServerConnection, StreamOwned};
use sha2::{Digest, Sha256};
use std::os::windows::io::AsRawHandle;
#[path = "application_surface_attach_images_tests.rs"]
mod attach_images;
#[path = "application_surface_resources_fixture.rs"]
mod fixture;
#[path = "application_surface_forms_tests.rs"]
mod forms;
#[path = "application_surface_inline_tests.rs"]
mod inline;
#[path = "application_surface_javascript_tests.rs"]
mod javascript;
#[path = "application_surface_javascript_input_tests.rs"]
mod javascript_input;
#[path = "application_live_source_tests.rs"]
mod live_source;
#[path = "application_live_viewer_tests.rs"]
mod live_viewer;
#[path = "application_surface_real_loom_tests.rs"]
mod real_loom;

fn run(corrupt: bool, close_before_delivery: bool, encoding: Option<&str>) {
    let _com = Apartment::new().unwrap();
    let (files, store, _) = Files::new();
    let (listener, client, server, pem) = fixture_with_pem();
    // Client 的 origin 私有；由已知 listener 端口构造完全相同的规范 origin。
    let origin = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let bytes: Vec<[u8; 4]> = if corrupt || close_before_delivery {
        vec![[1, 2, 3, 255]]
    } else {
        vec![[1, 2, 3, 255], [4, 5, 6, 255]]
    };
    let bytes: Vec<Vec<u8>> = bytes
        .into_iter()
        .map(|color| {
            if encoding.is_none() {
                return color.to_vec();
            }
            let mut bytes = Vec::new();
            if encoding == Some("image/webp") {
                image_webp::WebPEncoder::new(&mut bytes)
                    .encode(&color, 1, 1, image_webp::ColorType::Rgba8)
                    .unwrap();
            } else {
                let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&color)
                    .unwrap();
            }
            bytes
        })
        .collect();
    let sizes: Vec<_> = bytes.iter().map(Vec::len).collect();
    let ids: Vec<String> = bytes
        .iter()
        .map(|b| format!("sha256:{:x}", Sha256::digest(b)))
        .collect();
    let expected = ids.clone();
    let peer = std::thread::spawn(move || {
        for index in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(server.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            if index == 0 {
                assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                reply(
                    &mut stream,
                    200,
                    br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                );
            } else {
                assert!(head.starts_with("POST /v1/device-sessions "));
                reply(&mut stream,200,br#"{"deviceId":"device-old","token":"configured-token","expiresAtMs":18446744073709551615}"#);
            }
        }
        if !corrupt && !close_before_delivery {
            let mut stream = StreamOwned::new(
                ServerConnection::new(server.clone()).unwrap(),
                accept(&listener),
            );
            assert!(request(&mut stream).0.starts_with("GET /v1/live/sessions "));
            reply(
                &mut stream,
                200,
                br#"{"protocolVersion":"loom.live.v1","sessions":[]}"#,
            );
        }
        for (id, body) in expected.iter().zip(bytes) {
            let mut stream = StreamOwned::new(
                ServerConnection::new(server.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.starts_with(&format!("GET /v1/surfaces/resources/{} ", &id[7..])));
            let lower = head.to_ascii_lowercase();
            assert!(lower.contains("authorization: device configured-token"));
            assert!(lower.contains("x-loom-surface-lease: lease:"));
            assert!(lower.contains("x-loom-device-nonce:"));
            reply(&mut stream, 200, if corrupt { b"bad!" } else { &body });
        }
        listener
    });
    unsafe {
        let (_class, mut app) = fixture::application();
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.authorization_owner
            .start(crate::device_authorization_runtime::Task::Renew { client, store })
            .unwrap();
        {
            let handle = app.authorization_owner.wait_handles().next().unwrap();
            assert_eq!(
                windows::Win32::System::Threading::WaitForSingleObject(
                    HANDLE(handle.as_raw_handle()),
                    5000
                ),
                WAIT_OBJECT_0
            );
        }
        assert!(matches!(
            app.authorization_owner.poll().as_slice(),
            [crate::device_authorization_owner::Event::Authorized { .. }]
        ));
        let now = crate::desktop::surface_windows::now_ms().unwrap();
        let window = if let Some(mime) = encoding {
            fixture::window_images(&mut app, &origin, now, &ids, &sizes, mime)
        } else {
            fixture::window(&mut app, &origin, now, &ids)
        };
        app.surface_windows
            .insert(window)
            .unwrap_or_else(|_| panic!("window capacity"));
        if !corrupt && !close_before_delivery {
            let occupied: Vec<_> = (0..4)
                .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
                .collect();
            app.poll_remote_live_with(false, |o| fixture::config(o, files.root()))
                .unwrap();
            assert_eq!(app.remote_sessions.wait_handles().count(), 0);
            drop(occupied); // WouldBlock 没有消费候选；容量恢复后仍能启动第一张图。
        }
        let _catalog_competitors = if !corrupt && !close_before_delivery {
            let occupied: Vec<_> = (0..3)
                .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
                .collect();
            let lease = app.authorization_owner.lease(&origin, now).unwrap();
            app.remote_sessions
                .discover(fixture::config(&origin, files.root()).unwrap(), lease)
                .unwrap();
            fixture::wait(&app);
            // 已完成目录仍持有第四槽。完整 poll 必须先回收目录，再启动图片；
            // 否则图片遇 Busy，目录随后消耗最后完成信号，排队项没有后续唤醒。
            occupied
        } else {
            Vec::new()
        };
        app.poll_remote_live_with(false, |o| fixture::config(o, files.root()))
            .unwrap();
        // 第一轮只起一个任务，随后完成句柄驱动下一请求，无额外用户消息。
        fixture::wait(&app);
        if close_before_delivery {
            app.surface_windows.close_all().unwrap();
        }
        if corrupt {
            assert!(app
                .poll_remote_live_with(false, |o| fixture::config(o, files.root()))
                .is_err());
        } else if close_before_delivery {
            // 仅验证生产资源取消/安装段；完整生命周期退出会另发 inactive HTTP。
            app.remote_sessions.cancel_surface_resources(|_, _| true);
            assert!(app.remote_sessions.poll_surfaces().is_empty());
            app.poll_surface_images_with(|o| fixture::config(o, files.root()))
                .unwrap();
        } else {
            app.poll_remote_live_with(false, |o| fixture::config(o, files.root()))
                .unwrap();
            assert_eq!(
                app.surface_windows
                    .iter_mut()
                    .next()
                    .unwrap()
                    .installed_images_for_test(),
                1
            );
            fixture::wait(&app);
            app.poll_remote_live_with(false, |o| fixture::config(o, files.root()))
                .unwrap();
        }
        let listener = peer.join().unwrap();
        for _ in 0..5 {
            if !close_before_delivery {
                app.poll_remote_live_with(false, |o| fixture::config(o, files.root()))
                    .unwrap();
            }
        }
        assert!(matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        let window = app.surface_windows.iter_mut().next().unwrap();
        assert_eq!(
            window.installed_images_for_test(),
            if corrupt || close_before_delivery {
                0
            } else {
                2
            }
        );
        if !corrupt && !close_before_delivery {
            SendMessageW(window.edit_test_parent(), WM_PAINT, None, None);
            assert!(window.take_error().is_none());
            assert_eq!(window.uploaded_images_for_test(), 2);
        }
        // 不伪造远端停用 ACK：此夹具仅拥有资源 GET 服务端，显式关闭本地窗口/授权并 drain。
        app.surface_windows.close_all().unwrap();
        app.authorization_owner.close();
        app.remote_sessions.close();
        assert!(app.remote_sessions.poll_surfaces().is_empty());
        assert!(app.remote_sessions.finished());
    }
}
#[test]
fn application_images_poll_fetches_installs_and_continues_on_completion_without_input() {
    run(false, false, None);
}
#[test]
fn application_images_failed_get_is_not_repeated_on_later_polls() {
    run(true, false, None);
}
#[test]
fn application_images_closed_window_discards_completed_get() {
    run(false, true, None);
}
#[test]
fn application_images_verified_resource_sniffs_png_despite_jpeg_mime() {
    run(false, false, Some("IMAGE/JPEG"));
}
#[test]
fn application_images_webp_resource_child_installs_and_paints() {
    run(false, false, Some("image/webp"));
}
