//! 内联图片通过真实授权/完整应用轮询；图片阶段不允许准备网络配置。
use super::*;
fn png_source() -> String {
    use base64::Engine;
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[255, 0, 0, 255, 0, 255, 0, 128])
            .unwrap();
    }
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}
fn run_inline(missing_resource: bool, corrupt: bool, encoding: &str, mime: Option<&str>) {
    let _com = Apartment::new().unwrap();
    let (_files, store, _) = Files::new();
    let (listener, client, server, _) = fixture_with_pem();
    let origin = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
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
                reply(&mut stream, 200, br#"{"deviceId":"device-old","token":"configured-token","expiresAtMs":18446744073709551615}"#);
            }
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
        let source = if matches!(encoding, "webp" | "webp-p3") {
            use base64::Engine;
            let bytes = if encoding == "webp-p3" {
                crate::image_decode::Image::webp_p3_for_test()
            } else {
                crate::image_decode::Image::webp_for_test()
            };
            format!(
                "data:image/webp;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )
        } else if encoding == "svg-marker" {
            use base64::Engine;
            format!("data:image/svg+xml;base64,{}",base64::engine::general_purpose::STANDARD.encode("<svg xmlns='http://www.w3.org/2000/svg' width='2' height='1'><defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='1' markerHeight='1'><rect width='1' height='1' fill='red'/></marker></defs><path d='M0 0H1' fill='none' marker-end='url(#m)'/></svg>"))
        } else if encoding == "svg-pattern" {
            use base64::Engine;
            format!("data:image/svg+xml;base64,{}",base64::engine::general_purpose::STANDARD.encode("<svg xmlns='http://www.w3.org/2000/svg' width='2' height='1'><defs><pattern id='p' patternUnits='userSpaceOnUse' width='2' height='1'><rect width='1' height='1' fill='red'/></pattern></defs><rect width='2' height='1' fill='url(#p)'/></svg>"))
        } else if encoding == "svg-symbol" {
            use base64::Engine;
            format!("data:image/svg+xml;base64,{}",base64::engine::general_purpose::STANDARD.encode("<svg xmlns='http://www.w3.org/2000/svg' width='2' height='1'><defs><symbol id='s' viewBox='0 0 1 1'><rect width='1' height='1'/></symbol></defs><use href='#s' width='1' height='1' fill='red'/><use href='#s' x='1' width='1' height='1' fill='blue'/></svg>"))
        } else if encoding == "svg-mask" {
            use base64::Engine;
            format!("data:image/svg+xml;base64,{}",base64::engine::general_purpose::STANDARD.encode("<svg xmlns='http://www.w3.org/2000/svg' width='4' height='1'><defs><mask id='m'><rect width='2' height='1' fill='white'/></mask></defs><rect width='4' height='1' fill='red' mask='url(#m)'/></svg>"))
        } else if encoding == "svg-nested" {
            use base64::Engine;
            format!("data:image/svg+xml;base64,{}",base64::engine::general_purpose::STANDARD.encode("<svg xmlns='http://www.w3.org/2000/svg' width='80' height='40'><svg x='10' y='5' width='20' height='10'><rect width='100' height='100' fill='red'/></svg></svg>"))
        } else if encoding == "svg-text-length" {
            use base64::Engine;
            let svg = concat!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='48'>",
                "<text x='20' y='28' font-family='Consolas' font-size='16' ",
                "direction='rtl' text-anchor='end' textLength='80' ",
                "lengthAdjust='spacingAndGlyphs'>FH</text></svg>"
            );
            format!(
                "data:image/svg+xml;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(svg)
            )
        } else if encoding == "svg-text" {
            use base64::Engine;
            format!("data:image/svg+xml;base64,{}",base64::engine::general_purpose::STANDARD.encode("<svg xmlns='http://www.w3.org/2000/svg' width='128' height='48'><text y='28' font-size='24'>Hello 中文</text></svg>"))
        } else if encoding == "svg-fraction" {
            "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='2.5' height='1.5'%3E%3Crect width='1' height='1' fill='red'/%3E%3C/svg%3E".to_owned()
        } else if encoding == "svg" {
            "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='2' height='1'%3E%3Crect width='2' height='1' fill='red'/%3E%3C/svg%3E".to_owned()
        } else if encoding == "jpeg" {
            use base64::Engine;
            format!(
                "data:image/jpeg;base64,{}",
                base64::engine::general_purpose::STANDARD
                    .encode(crate::image_decode::Image::oriented_jpeg_for_test())
            )
        } else if corrupt {
            "data:image/png;base64,YmFk".to_owned()
        } else {
            png_source()
        };
        let source = mime.map_or_else(
            || source.clone(),
            |mime| format!("data:{mime},{}", source.split_once(',').unwrap().1),
        );
        let mut props = serde_json::json!({"src": source});
        if missing_resource {
            props["resourceId"] = "missing-resource".into();
        }
        props["alt"] = "测试图片".into();
        let children = vec![
            serde_json::json!({"id":"inline","type":"image","props":props,"layout":{"width":32,"height":32}}),
        ];
        let window = fixture::window_with_sources(&mut app, &origin, now, children, vec![]);
        app.surface_windows
            .insert(window)
            .unwrap_or_else(|_| panic!("capacity"));
        {
            let window = app.surface_windows.iter_mut().next().unwrap();
            assert_eq!(
                window.image_placeholder_for_test("inline"),
                (!missing_resource).then_some(true)
            );
            SendMessageW(window.edit_test_parent(), WM_PAINT, None, None);
            assert!(window.take_error().is_none());
        }
        app.poll_remote_live_with(false, |_| panic!("inline must not prepare HTTP Config"))
            .unwrap();
        if !missing_resource {
            fixture::wait(&app);
            let result =
                app.poll_remote_live_with(false, |_| panic!("inline must not prepare HTTP Config"));
            assert_eq!(result.is_err(), corrupt);
        }
        for _ in 0..5 {
            app.poll_remote_live_with(false, |_| panic!("inline must not prepare HTTP Config"))
                .unwrap();
        }
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        let window = app.surface_windows.iter_mut().next().unwrap();
        let expected = usize::from(!missing_resource && !corrupt);
        assert_eq!(window.installed_images_for_test(), expected);
        if encoding == "jpeg" {
            assert_eq!(
                window.installed_image_size_for_test("inline"),
                Some([32., 48.])
            );
        }
        if encoding == "svg-fraction" {
            assert_eq!(
                window.installed_image_size_for_test("inline"),
                Some([2.5, 1.5])
            );
        }
        if encoding == "svg-text-length" {
            assert_eq!(
                window.installed_image_size_for_test("inline"),
                Some([128., 48.])
            );
        }
        assert_eq!(
            window.image_placeholder_for_test("inline"),
            (!missing_resource).then_some(corrupt)
        );
        SendMessageW(window.edit_test_parent(), WM_PAINT, None, None);
        assert!(window.take_error().is_none());
        if expected == 1 {
            SendMessageW(window.edit_test_parent(), WM_PAINT, None, None);
            assert!(window.take_error().is_none());
            assert_eq!(window.uploaded_images_for_test(), 1);
        }
        let listener = peer.join().unwrap();
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
        app.surface_windows.close_all().unwrap();
        app.authorization_owner.close();
        app.remote_sessions.close();
        assert!(app.remote_sessions.poll_surfaces().is_empty());
        assert!(app.remote_sessions.finished());
    }
}
#[test]
fn application_images_inline_png_installs_without_config_or_resource_get() {
    run_inline(false, false, "png", None);
}
#[test]
fn application_images_missing_resource_never_falls_back_to_inline() {
    run_inline(true, false, "png", None);
}
#[test]
fn application_images_corrupt_inline_is_not_retried() {
    run_inline(false, true, "png", None);
}
#[test]
fn application_images_inline_oriented_jpeg_installs_and_paints() {
    run_inline(false, false, "jpeg", None);
}
#[test]
fn application_images_inline_sniffed_png_installs_and_paints() {
    for mime in [
        "image/jpeg;base64",
        "image/unknown;base64",
        "image/PNG; charset=binary;base64",
    ] {
        run_inline(false, false, "png", Some(mime));
    }
}
#[test]
fn application_images_webp_inline_child_installs_and_paints() {
    run_inline(false, false, "webp", None);
}
#[test]
fn application_images_webp_p3_child_installs_and_paints() {
    run_inline(false, false, "webp-p3", None);
}
#[test]
fn application_images_svg_inline_child_installs_and_paints() {
    run_inline(false, false, "svg", None);
    run_inline(false, false, "svg-fraction", None);
    run_inline(false, false, "svg-text", None);
    run_inline(false, false, "svg-nested", None);
    run_inline(false, false, "svg-mask", None);
    run_inline(false, false, "svg-symbol", None);
    run_inline(false, false, "svg-pattern", None);
    run_inline(false, false, "svg-marker", None);
}

#[test]
fn application_images_svg_text_length_direction_installs_and_paints() {
    run_inline(false, false, "svg-text-length", None);
}
