//! 显式真实Loom LAN Source联合消费者，沿实际系统菜单/Application，不是真实桌面捕获或双普通产品。
use super::*;
use base64::Engine;
#[path = "application_live_source_joint_observe.rs"]
mod observe;
#[path = "application_live_source_joint_reconnect.rs"]
mod reconnect;
#[test]
#[ignore = "explicit real Loom Source publication and cleanup"]
fn source_menu_real_loom_lan() {
    run(0);
}
#[test]
#[ignore = "explicit real Loom Source control and original native input worker"]
fn source_input_menu_real_loom_lan() {
    run(1);
}
#[test]
#[ignore = "explicit real Loom Source UIA observation and cleanup"]
fn source_observation_menu_real_loom_lan() {
    run(2);
}
#[test]
#[ignore = "explicit real Loom LAN H264 Source policy and native viewer joint"]
fn source_h264_menu_real_loom_lan() {
    run(3);
}
#[test]
#[ignore = "explicit real Loom LAN continuously refreshed Tile Source fixture"]
fn source_tile_menu_real_loom_lan() {
    run(4);
}
fn run(mode: u8) {
    let interactive = mode == 1;
    let input =
        std::path::PathBuf::from(std::env::var_os("OTTER_LIVE_SOURCE_JOINT_INPUT").unwrap());
    assert!(input.is_absolute() && std::fs::metadata(&input).unwrap().len() <= 64 * 1024);
    let data: serde_json::Value =
        serde_json::from_slice(&zeroize::Zeroizing::new(std::fs::read(&input).unwrap())).unwrap();
    let root = input.parent().unwrap();
    let origin = data["origin"].as_str().unwrap();
    let url = url::Url::parse(origin).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    let profile = root.join("native-source-profile");
    std::fs::create_dir(&profile).unwrap();
    std::fs::write(
        profile.join("device-identity.json"),
        serde_json::to_vec(&data["identity"]).unwrap(),
    )
    .unwrap();
    let cert = base64::engine::general_purpose::STANDARD
        .decode(data["certificate"].as_str().unwrap())
        .unwrap();
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(rustls::pki_types::CertificateDer::from(cert))
        .unwrap();
    std::fs::write(
        root.join("fixture-ca.pem"),
        format!(
            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
            data["certificate"].as_str().unwrap()
        ),
    )
    .unwrap();
    let client = crate::device_session_client::Client::new(
        origin,
        roots,
        vec![format!("127.0.0.1:{}", url.port().unwrap())
            .parse()
            .unwrap()],
    )
    .unwrap();
    let config = |origin: &str| {
        crate::device_authorization_config::Config::from_lookup(origin, |key| match key {
            "OTTER_NATIVE_PROFILE_DIR" => Some(profile.clone().into_os_string()),
            "OTTER_LOOM_TLS_ORIGIN" => Some(origin.into()),
            "OTTER_LOOM_TLS_CA_FILE" => Some(root.join("fixture-ca.pem").into_os_string()),
            _ => None,
        })
    };
    let _com = Apartment::new().unwrap();
    unsafe {
        let target = interactive.then(super::interaction::oracle::Window::new);
        let observed = (mode == 2).then(crate::live_source_uia_test_window::Window::new);
        let (_class, mut app) = fixture::application();
        app.authorization.borrow_mut().origin = Some(origin.into());
        app.authorization_owner
            .start(crate::device_authorization_runtime::Task::Renew {
                client,
                store: crate::device_identity_store::Store::new(&profile).unwrap(),
            })
            .unwrap();
        for handle in app.authorization_owner.wait_handles() {
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
        let color = Arc::new(AtomicUsize::new(0));
        let active = if mode == 4 {
            capture_stream(&mut app, color.clone())
        } else {
            capture_size(&mut app, if mode == 3 { [64, 64] } else { [2, 2] })
        };
        if let Some(target) = &target {
            app.live_sessions[0]
                .test_input_target(crate::live_window_target::Target::bind(target.hwnd).unwrap());
        }
        if let Some(window) = &observed {
            app.live_sessions[0]
                .test_input_target(crate::live_window_target::Target::bind(window.hwnd).unwrap());
        }
        let identity = app.live_sessions[0].input_identity();
        let hwnd = app.live_sessions[0].group_dialog_owner().unwrap();
        let command = if interactive {
            crate::desktop::composition::live::window::publication::PUBLISH_CONTROL
        } else if mode == 2 {
            crate::desktop::composition::live::window::publication::PUBLISH_OBSERVE
        } else {
            PUBLISH
        };
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(command)), None);
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            app.poll_live_sources_with(&mut |origin| config(origin));
            if app.remote_sessions.source_status(&identity) == Some((true, false)) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "real source publication: {}",
                app.authorization.borrow().live.status
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        std::fs::write(root.join("source-ready.json"),serde_json::to_vec(&serde_json::json!({"session":app.remote_sessions.source_session(&identity).unwrap(),"menu":true,"application":true})).unwrap()).unwrap();
        let mut progress = observe::Progress::default();
        let mut recovery = reconnect::Progress::default();
        let mut changed = false;
        while !root.join("source-stop").exists() {
            if mode == 4 && root.join("source-pixels-change").exists() {
                color.store(1, Ordering::Release);
            }
            app.poll_live_sources_with(&mut |origin| config(origin));
            for source in &mut app.live_sessions {
                source.poll();
            }
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            recovery.poll(root, &mut app, &identity, hwnd);
            if let Some(target) = &target {
                let source = &app.live_sessions[0];
                // Bridge在发布期间保留控制资格；本门核原输入线程已回收及真实cleanup，不能等Bridge退休。
                let released = !source.input_enabled()
                    && source.input_completion_handle().is_none()
                    && source
                        .input_cleanup()
                        .is_some_and(|r| r.failed == 0 && r.skipped == 0)
                    && app.remote_sessions.source_status(&identity) == Some((true, false));
                progress.poll(root, target, released);
            }
            if !changed && root.join("source-change").exists() {
                observed.as_ref().unwrap().value("value-eight");
                changed = true;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(STOP)), None);
        loop {
            app.poll_live_sources_with(&mut |origin| config(origin));
            for source in &mut app.live_sessions {
                source.poll();
            }
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if app.remote_sessions.source_status(&identity).is_none()
                && (!interactive || app.live_sessions[0].remote_input_idle())
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(app.authorization.borrow().live.status.contains("Confirmed"));
        assert!(app.live_sessions[0].is_alive());
        assert!(app.remote_sessions.finished());
        if let Some(target) = &target {
            let messages = target.messages();
            assert!(messages.iter().any(|&(m, k, _)| m == WM_KEYUP && k == 65));
            assert!(messages.iter().any(|&(m, _, _)| m == WM_LBUTTONUP));
            assert_ne!(GetForegroundWindow(), target.hwnd);
            std::fs::write(root.join("source-input-released.json"),br#"{"actualKeyUp":65,"actualMouseUp":true,"inputOwnerReclaimed":true,"sourceNotForegroundAtEnd":true}"#).unwrap();
        }
        app.shutdown_live().unwrap();
        assert_eq!(active.load(Ordering::Acquire), 0);
        std::fs::write(root.join("source-complete.json"),br#"{"sourceClosed":true,"capturePreservedUntilShutdown":true,"ownersReclaimed":true,"ordinaryDualProduct":false,"physicalCrossDevice":false}"#).unwrap();
    }
}
