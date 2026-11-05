//! 实际身份文件、TLS、Windows 线程完成句柄的组合验证；不连接在线 Loom。
use super::*;
use crate::device_authorization_runtime::{Error as RuntimeError, Output, Runtime, Task};
use crate::device_identity_store::Store;
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};

fn wait(runtime: &Runtime) {
    let handle = runtime.wait_handle().unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}

#[test]
fn actual_renewal_delivers_session_or_suppresses_completed_session_after_stop() {
    for stopped in [false, true] {
        let (_files, store, _loaded) = super::pairing::Files::new();
        let (listener, client, config) = fixture();
        let origin = client.origin.clone();
        let peer = std::thread::spawn(move || {
            for index in 0..2 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(config.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, body) = request(&mut stream);
                assert_eq!(body["deviceId"], "device-old");
                if index == 0 {
                    assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                    reply(
                        &mut stream,
                        200,
                        br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                    );
                } else {
                    assert!(head.starts_with("POST /v1/device-sessions "));
                    reply(&mut stream, 200, br#"{"deviceId":"device-old","token":"runtime-secret","expiresAtMs":18446744073709551615}"#);
                }
            }
        });
        let mut runtime = Runtime::start(Task::Renew { client, store }).unwrap();
        wait(&runtime);
        if stopped {
            runtime.request_stop();
        }
        match runtime.try_join().unwrap() {
            Ok(Output::Session(session)) if !stopped => {
                assert_eq!(session.token_for(&origin, 0), Ok("runtime-secret"))
            }
            Err(RuntimeError::Stopped) if stopped => {}
            _ => panic!("unexpected authorization result"),
        }
        assert!(runtime.wait_handle().is_none());
        peer.join().unwrap();
    }
}

#[test]
fn actual_pairing_task_only_registers_and_returns_binding_result() {
    let (_files, store, loaded) = super::pairing::Files::new();
    let public = loaded.identity().public_key().unwrap();
    let (listener, client, config) = fixture();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/devices/requests "));
        assert_eq!(body["publicKey"], public);
        let body = serde_json::to_vec(
            &serde_json::json!({"pending":[{"id":"new-device","publicKey":public}]}),
        )
        .unwrap();
        reply(&mut stream, 200, &body);
        assert!(stream.read(&mut [0]).map_or(true, |n| n == 0));
    });
    let mut runtime = Runtime::start(Task::RequestPairing {
        client,
        store,
        name: "测试电脑".into(),
    })
    .unwrap();
    wait(&runtime);
    assert!(
        matches!(runtime.try_join(), Some(Ok(Output::Pairing { device, changed: true })) if device == "new-device")
    );
    peer.join().unwrap();
}

#[test]
fn renewal_and_approval_missing_identity_never_create_or_connect() {
    for approval in [false, true] {
        let path = std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(
            format!(
                "otter-missing-auth-{:?}",
                unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
            ),
        );
        assert!(!path.exists());
        let store = Store::new(&path).unwrap();
        let (listener, client, _) = fixture();
        let task = if approval {
            Task::AwaitApproval { client, store }
        } else {
            Task::Renew { client, store }
        };
        let mut runtime = Runtime::start(task).unwrap();
        wait(&runtime);
        assert!(matches!(
            runtime.try_join(),
            Some(Err(RuntimeError::Store(
                crate::device_identity_store::Error::Missing
            )))
        ));
        assert!(!path.exists());
        assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
    }
}

#[test]
fn configured_renewal_loads_scoped_ca_and_resolves_in_the_background_task() {
    use crate::device_authorization_config::{Action, Config};
    let (files, _, _) = super::pairing::Files::new();
    let (listener, client, server_config, pem) = fixture_with_pem();
    let origin = client.origin.clone();
    let ca_file = files.root().join("fixture-ca.pem");
    std::fs::write(&ca_file, pem.as_bytes()).unwrap();
    let config = Config::from_lookup(&origin, |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(files.root().as_os_str().to_owned()),
        "OTTER_LOOM_TLS_ORIGIN" => Some(origin.clone().into()),
        "OTTER_LOOM_TLS_CA_FILE" => Some(ca_file.as_os_str().to_owned()),
        _ => None,
    })
    .unwrap();
    let peer = std::thread::spawn(move || {
        for index in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(server_config.clone()).unwrap(),
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
    });
    let mut runtime = Runtime::start(Task::Configured {
        config,
        action: Action::Renew,
    })
    .unwrap();
    wait(&runtime);
    match runtime.try_join().unwrap().unwrap() {
        Output::Session(session) => {
            assert_eq!(session.token_for(&origin, 0), Ok("configured-token"))
        }
        _ => panic!("unexpected pairing result"),
    }
    peer.join().unwrap();
}
