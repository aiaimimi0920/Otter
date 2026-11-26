//! Explicit isolated real-daemon gate: real pairing/device sessions/attach/ACK, without injecting Outcome or credentials.
use super::*;
use crate::device_authorization_config::Action;
use crate::device_authorization_owner::Event;
use std::{
    io::{self, Read, Write},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
use windows::Win32::{
    Storage::FileSystem::{FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT},
    System::Threading::WaitForSingleObject,
};

const INPUT_LIMIT: u64 = 64 * 1024;

fn read_input(root: &Path) -> io::Result<serde_json::Value> {
    // Check the opened file itself; a descriptor link must not redirect outside the fixture.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(root.join("test-input.json"))?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
        || metadata.len() > INPUT_LIMIT
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid Loom fixture input file",
        ));
    }
    let mut bytes = Vec::new();
    file.take(INPUT_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > INPUT_LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Loom fixture input exceeds its byte limit",
        ));
    }
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

#[test]
fn descriptor_accepts_limit_and_rejects_larger_input() {
    let root = crate::test_temp_root().join(format!("otter-loom-descriptor-{:?}", unsafe {
        windows::Win32::System::Com::CoCreateGuid().unwrap()
    }));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("test-input.json");
    let mut bytes = br#"{"nodeId":"test"}"#.to_vec();
    bytes.resize(INPUT_LIMIT as usize, b' ');
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(read_input(&root).unwrap()["nodeId"], "test");
    bytes.push(b' ');
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(
        read_input(&root).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}

fn record(root: &Path, name: &str, value: serde_json::Value) {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))
        .unwrap();
    file.write_all(&serde_json::to_vec(&value).unwrap())
        .unwrap();
    file.sync_all().unwrap();
}
fn authorize(app: &mut Application, origin: &str, profile: &Path, action: Action) -> Event {
    app.authorization_owner
        .start(crate::device_authorization_runtime::Task::Configured {
            config: fixture::config(origin, profile).unwrap(),
            action,
        })
        .unwrap();
    for handle in app.authorization_owner.wait_handles() {
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 30000) },
            WAIT_OBJECT_0
        );
    }
    let mut events = app.authorization_owner.poll();
    assert_eq!(events.len(), 1);
    let event = events.pop().unwrap();
    if let Event::Failed { error, .. } = &event {
        panic!("real Loom authorization failed: {error:?}");
    }
    event
}
fn wait_remote(app: &Application, phase: &str) {
    let handles: Vec<_> = app.remote_sessions.wait_handles().collect();
    assert!(!handles.is_empty(), "{phase}: no pending task");
    let started = std::time::Instant::now();
    for handle in handles {
        // Real configuration includes DNS/multi-address fallback and a ten-second HTTP budget; do not reuse the synthetic single-address peer's five-second wait.
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 30000) },
            WAIT_OBJECT_0,
            "{phase}: whole worker did not finish"
        );
    }
    println!(
        "real-loom-application: phase={phase} worker-ms={}",
        started.elapsed().as_millis()
    );
}
unsafe fn offscreen(parent: HWND) {
    let mut rect = RECT::default();
    GetWindowRect(parent, &mut rect).unwrap();
    let x = GetSystemMetrics(SM_XVIRTUALSCREEN)
        .checked_sub(rect.right - rect.left + 64)
        .unwrap();
    let y = GetSystemMetrics(SM_YVIRTUALSCREEN)
        .checked_sub(rect.bottom - rect.top + 64)
        .unwrap();
    SetWindowPos(
        parent,
        None,
        x,
        y,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    )
    .unwrap();
}

#[test]
#[ignore = "requires a fresh explicitly provisioned real Loom TLS fixture under repository .tmp"]
fn isolated_real_loom_pairs_mounts_activates_and_closes() {
    let root = std::path::PathBuf::from(
        std::env::var_os("OTTER_REAL_LOOM_TEST_DIR").expect("explicit isolated fixture required"),
    );
    let root = crate::temp_root::fixture_directory(&root);
    let input = read_input(&root).expect("invalid isolated Loom test-input.json");
    let origin = input["origin"].as_str().unwrap();
    let url = url::Url::parse(origin).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("localhost"));
    let art = input["artId"].as_str().unwrap();
    let node = input["nodeId"].as_str().unwrap();
    let profile = root.join("otter-profile");
    assert!(
        !profile.join("device-identity.json").exists(),
        "fixture was already used"
    );
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut app = fixture::application();
        app.authorization.borrow_mut().origin = Some(origin.into());
        let device = match authorize(&mut app, origin, &profile, Action::RequestPairing) {
            Event::Pairing { device, .. } => device,
            _ => panic!("pairing did not return a pairing event"),
        };
        assert_ne!(device, "device-000-local");
        // The external fixture approves only this run's new ID; tokens always come from production signing/session paths and are never written to receipts.
        record(
            &root,
            "pairing-ready.json",
            serde_json::json!({"deviceId":device}),
        );
        match authorize(&mut app, origin, &profile, Action::AwaitApproval) {
            Event::Authorized {
                device: authorized, ..
            } => assert_eq!(authorized, device),
            _ => panic!("approval did not produce a real device session"),
        }
        super::super::super::super::surface_start::start_with(
            origin,
            art.into(),
            node.into(),
            &mut app.authorization_owner,
            &mut app.remote_sessions,
            &app.surface_windows,
            |requested| fixture::config(requested, &profile),
        )
        .unwrap();
        wait_remote(&app, "attach");
        app.poll_remote_live_with(false, |o| fixture::config(o, &profile))
            .unwrap();
        {
            let window = app
                .surface_windows
                .iter_mut()
                .next()
                .expect("attach was not admitted");
            assert!(window.is_alive() && !window.is_shown());
            assert!(window.lifecycle().is_none());
            assert_eq!(window.binding().device(), device);
            assert_eq!(window.binding().node(), node);
        }
        wait_remote(&app, "active");
        app.poll_remote_live_with(false, |o| fixture::config(o, &profile))
            .unwrap();
        let (parent, instance, attachment) = {
            let window = app.surface_windows.iter_mut().next().unwrap();
            assert!(window.lifecycle().unwrap().confirmed_active());
            let parent = window.edit_test_parent();
            offscreen(parent);
            let now = crate::desktop::surface_windows::now_ms().unwrap();
            window.show(now).unwrap();
            assert!(window.sync_input(true, true, now).unwrap());
            assert!(window.is_shown());
            assert_ne!(GetForegroundWindow(), parent);
            let mut rect = RECT::default();
            GetWindowRect(parent, &mut rect).unwrap();
            assert!(rect.right < GetSystemMetrics(SM_XVIRTUALSCREEN));
            assert!(rect.bottom < GetSystemMetrics(SM_YVIRTUALSCREEN));
            let edit = FindWindowExW(Some(parent), None, w!("EDIT"), None).unwrap();
            assert!(windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(edit).as_bool());
            let first = FindWindowExW(Some(parent), None, w!("COMBOBOX"), None).unwrap();
            let second = FindWindowExW(Some(parent), Some(first), w!("COMBOBOX"), None).unwrap();
            assert!(FindWindowExW(Some(parent), Some(second), w!("COMBOBOX"), None).is_err());
            for _ in 0..2 {
                SendMessageW(parent, WM_PAINT, None, None);
                assert!(window.take_error().is_none());
            }
            (
                parent,
                window.binding().instance().to_owned(),
                window.binding().attachment().to_owned(),
            )
        };
        app.surface_windows.close_all().unwrap();
        assert!(!IsWindow(Some(parent)).as_bool());
        app.poll_remote_live_with(false, |o| fixture::config(o, &profile))
            .unwrap();
        wait_remote(&app, "inactive");
        app.poll_remote_live_with(false, |o| fixture::config(o, &profile))
            .unwrap();
        assert!(app.surface_windows.iter_mut().next().is_none());
        let records = app.surface_windows.retired_records();
        assert_eq!(records.len(), 1);
        assert!(!records[0].uncertain() && !records[0].in_flight());
        let hint = serde_json::to_value(&records[0]).unwrap();
        assert_eq!(hint["binding"]["lifecycle"], "inactive");
        assert_eq!(hint["binding"]["instance"], instance);
        assert_eq!(hint["binding"]["attachment"], attachment);
        assert!(app.remote_sessions.wait_handles().next().is_none());
        app.authorization_owner.close();
        assert!(app.authorization_owner.finished());
        record(
            &root,
            "application-result.json",
            serde_json::json!({"deviceId":device,
            "instanceId":instance,"attachmentId":attachment,"origin":origin,
            "realDeviceSession":true,"realApplication":true,"paints":2,"controls":3,
            "closed":true,"inactiveAck":true,"javascript":false,"productExe":false}),
        );
    }
    println!("real-loom-application: paired=true device-session=true attach=true active=true controls=3 paints=2 inactive=true closed=true product-exe=false javascript=false");
}
