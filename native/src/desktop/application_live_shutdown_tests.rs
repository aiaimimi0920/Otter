use super::*;

#[test]
fn shutdown_drains_remote_control_owner_and_releases_wait_handles() {
    use crate::{
        desktop::composition::relay,
        device_authorization_lease::Grant,
        device_identity::Identity,
        device_session_attempt::Attempt,
        device_session_client::Client,
        live_relay_control_runtime::{Runtime, Target},
    };
    let mut identity = Identity::generate().unwrap();
    identity.bind("viewer:test").unwrap();
    let session = Attempt::new(&identity, "https://localhost:9")
        .unwrap()
        .challenge(
            200,
            br#"{"deviceId":"viewer:test","challengeId":"id","challenge":"value"}"#,
        )
        .unwrap()
        .finish(
            200,
            br#"{"deviceId":"viewer:test","token":"secret","expiresAtMs":99999}"#,
            0,
        )
        .unwrap();
    let mut grant = Grant::new(session);
    let lease = grant.lease("https://localhost:9", 0).unwrap();
    let catalog_lease = grant.lease("https://localhost:9", 0).unwrap();
    let surface_lease = grant.lease("https://localhost:9", 0).unwrap();
    drop(grant);
    let client = Client::new(
        "https://localhost:9",
        rustls::RootCertStore::empty(),
        vec!["127.0.0.1:9".parse().unwrap()],
    )
    .unwrap();
    let runtime = Runtime::start(
        client,
        lease,
        Target {
            session: "live:test".into(),
            surface: "surface:test".into(),
            attachment: "attach:test".into(),
            expected: None,
            selected: None,
        },
        |_| panic!("revoked callback"),
    )
    .unwrap();
    let mut remote = relay::sessions::Sessions::default();
    let binding=crate::surface_live_binding::Binding::from_mount(
        br#"{"instance":{"descriptor":{"instanceId":"surface:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"surface:test","deviceId":"viewer:test","hookNodeId":"node:test"},"lifecycle":"mounted","lifecycleRevision":1}}}}"#,
        "https://localhost:9","viewer:test","node:test").unwrap();
    remote
        .start_surface(
            crate::device_authorization_config::Config::new(
                "https://localhost:9",
                std::path::Path::new(
                    "C:/Users/Public/nas_home/AI/GameEditor/linshi/unused-relay-profile",
                ),
                "test",
            )
            .unwrap(),
            surface_lease,
            crate::surface_lifecycle_owner::Owner::new(binding),
            crate::surface_lifecycle_runtime::Action::Recover,
        )
        .unwrap_or_else(|e| panic!("{}", e.error));
    remote
        .discover(
            crate::device_authorization_config::Config::new(
                "https://localhost:9",
                std::path::Path::new(
                    "C:/Users/Public/nas_home/AI/GameEditor/linshi/unused-relay-profile",
                ),
                "test",
            )
            .unwrap(),
            catalog_lease,
        )
        .unwrap();
    remote.insert_for_test(
        "https://localhost:9".into(),
        relay::session::Session::from_runtime(runtime),
    );
    assert!(!remote.finished());
    drain(
        &mut Vec::new(),
        &mut Default::default(),
        &mut Default::default(),
        &mut remote,
    )
    .unwrap();
    assert!(remote.finished());
    assert_eq!(remote.wait_handles().count(), 0);
    assert!(remote.poll().is_empty());
    assert!(remote.take_surface_outcome().is_some());
    assert!(remote.take_surface_outcome().is_none());
}
use crate::{
    desktop::{composition::live::window::LiveWindow, Apartment, Window},
    live_job::LiveJob,
    live_texture::Texture,
};
use std::{
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

#[test]
fn shutdown_pumps_synchronous_worker_messages_and_preserves_quit() {
    let _apartment = Apartment::new().unwrap();
    let target = Window(unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("drain"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    });
    let active = Arc::new(AtomicUsize::new(0));
    let observed = Arc::new(AtomicUsize::new(0));
    let mut sessions = vec![waiting_session(target.0, &active, &observed)];
    unsafe {
        PostQuitMessage(7);
    }
    drain(
        &mut sessions,
        &mut Default::default(),
        &mut Default::default(),
        &mut Default::default(),
    )
    .unwrap();
    assert!(sessions.is_empty());
    assert_eq!(active.load(Ordering::Acquire), 0);
    assert_eq!(observed.load(Ordering::Acquire), 5);
    let mut message = MSG::default();
    assert!(unsafe { PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE) }.as_bool());
    assert_eq!(message.wParam.0, 7);
}

fn waiting_session(
    target: HWND,
    active: &Arc<AtomicUsize>,
    observed: &Arc<AtomicUsize>,
) -> Session {
    let hwnd = target.0 as usize;
    let delivered = observed.clone();
    let (entered, ready) = mpsc::channel();
    let (job, window) = LiveJob::<Texture>::spawn_bound(
        active,
        |consumer| LiveWindow::new(consumer, 128, 128).map_err(io::Error::other),
        move |source, _| {
            let (wake, wait) = mpsc::channel();
            source
                .on_idle(Arc::new(move || {
                    let _ = wake.send(());
                }))
                .unwrap();
            entered.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            let mut length = 0;
            let result = unsafe {
                SendMessageTimeoutW(
                    HWND(hwnd as *mut _),
                    WM_GETTEXTLENGTH,
                    WPARAM(0),
                    LPARAM(0),
                    SMTO_ABORTIFHUNG | SMTO_BLOCK | SMTO_ERRORONEXIT,
                    3000,
                    Some(&mut length),
                )
            };
            if result.0 == 0 {
                return Err("UI did not pump shutdown message".into());
            }
            delivered.store(length, Ordering::Release);
            Ok(())
        },
    )
    .unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    Session::new(job, window)
}

#[test]
fn confirmed_system_end_still_pumps_worker_and_destroys_live_owner() {
    use crate::desktop::session_end::{dispatch_active, with_owner, Control};
    let _apartment = Apartment::new().unwrap();
    let control = std::rc::Rc::new(Control::default());
    with_owner(&control, || unsafe {
        let target = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("drain"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let active = Arc::new(AtomicUsize::new(0));
        let observed = Arc::new(AtomicUsize::new(0));
        let mut sessions = vec![waiting_session(target.0, &active, &observed)];
        assert!(dispatch_active(
            target.0,
            &std::cell::Cell::new(false),
            WM_ENDSESSION,
            WPARAM(1),
            LPARAM(0)
        )
        .is_some());
        assert!(control.ended());
        drain(
            &mut sessions,
            &mut Default::default(),
            &mut Default::default(),
            &mut Default::default(),
        )
        .unwrap();
        assert!(sessions.is_empty());
        assert_eq!(active.load(Ordering::Acquire), 0);
        assert_eq!(observed.load(Ordering::Acquire), 5);
        let mut message = MSG::default();
        assert!(PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE).as_bool());
        assert_eq!(message.wParam.0, 0);
    });
}

#[test]
fn application_error_exit_still_pumps_and_reaps_live_workers() {
    use crate::desktop::document_window::DocumentWindow;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    struct Class(HINSTANCE);
    impl Drop for Class {
        fn drop(&mut self) {
            unsafe {
                let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
            }
        }
    }
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Native.Workspace"),
                lpfnWndProc: Some(crate::desktop::procedure::window_proc),
                ..Default::default()
            }),
            0
        );
        let _class = Class(instance);
        let target = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("drain"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let document =
            DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
        let mut app = Application::new(instance, document);
        let active = Arc::new(AtomicUsize::new(0));
        let observed = Arc::new(AtomicUsize::new(0));
        app.live_sessions
            .push(waiting_session(target.0, &active, &observed));
        PostQuitMessage(7);
        let result = app.run(None);
        assert!(result
            .unwrap_err()
            .message()
            .contains("Native message callback failed"));
        assert_eq!(
            observed.load(Ordering::Acquire),
            5,
            "Error return bypassed UI cleanup pump"
        );
        assert_eq!(active.load(Ordering::Acquire), 0);
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE).as_bool() {}
    }
}

#[test]
fn shutdown_drains_capture_and_real_recovery_registration_together() {
    use crate::{
        live_visibility_owner::Owner,
        live_window_target::{snapshot::Snapshot, Target},
    };
    use windows::Win32::System::Threading::WaitForSingleObject;
    let _apartment = Apartment::new().unwrap();
    let window = Window(unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("STATIC"),
            w!("drain"),
            WS_POPUP,
            20,
            20,
            32,
            32,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    });
    let target = Target::bind(window.0).unwrap();
    let before = Snapshot::read(&target).unwrap().recovery_record().unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let observed = Arc::new(AtomicUsize::new(0));
    let mut visibility = Owner::fixture();
    let mut session = waiting_session(window.0, &active, &observed);
    session.protect(visibility.acquire(target.clone()).unwrap());
    let end = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        visibility.poll().unwrap();
        assert!(session.poll().is_none());
        if Snapshot::read(&target)
            .unwrap()
            .layered
            .is_some_and(|value| value.alpha == 1)
        {
            break;
        }
        assert!(std::time::Instant::now() < end);
        let handle = visibility
            .completion_handle()
            .or_else(|| visibility.child_handle())
            .unwrap();
        let result = unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 1000) };
        assert!(result == WAIT_OBJECT_0 || result == WAIT_TIMEOUT);
    }
    let mut sessions = vec![session];
    drain(
        &mut sessions,
        &mut visibility,
        &mut Default::default(),
        &mut Default::default(),
    )
    .unwrap();
    assert!(visibility.finished());
    assert!(sessions.is_empty());
    assert_eq!(active.load(Ordering::Acquire), 0);
    assert_eq!(observed.load(Ordering::Acquire), 5);
    assert_eq!(
        Snapshot::read(&target).unwrap().recovery_record().unwrap(),
        before
    );
}
