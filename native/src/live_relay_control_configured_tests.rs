//! Configured Live startup reuses authorization's background trust loading/DNS without creating device identity.
use super::*;
use crate::device_authorization_config::{Config, Error as PrepareError};

#[test]
fn configured_runtime_loads_scoped_ca_and_runs_join_without_identity_store() {
    let (listener, client, tls, pem) = fixture_with_pem();
    let root = crate::test_temp_root().join(format!(
        "otter-live-config-{:?}",
        unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let ca = root.join("ca.pem");
    std::fs::write(&ca, pem).unwrap();
    let profile = root.join("profile-not-created");
    let config = Config::from_lookup(&client.origin, |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(profile.clone().into_os_string()),
        "OTTER_LOOM_TLS_ORIGIN" => Some(client.origin.clone().into()),
        "OTTER_LOOM_TLS_CA_FILE" => Some(ca.clone().into_os_string()),
        _ => None,
    })
    .unwrap();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let peer = std::thread::spawn(move || joined(&listener, &tls));
    let mut runtime = Runtime::start_configured(config, lease, target(), |_| Err(())).unwrap();
    // System localhost may first resolve to unserved IPv6; each of four requests has its own two-second address-fallback budget.
    // Wait for the whole join chain here; this does not relax cancellation-latency assertions.
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(runtime.wait_handle().unwrap().as_raw_handle()),
                20000,
            )
        },
        WAIT_OBJECT_0
    );
    assert_eq!(
        runtime.try_join(),
        Some(Exit::Control(OwnerError::Consumer))
    );
    assert!(runtime.try_ready().is_none());
    assert!(!profile.exists());
    peer.join().unwrap();
}

#[test]
fn configured_wrong_origin_expired_or_revoked_lease_fails_before_ca_or_dns() {
    for mode in 0..3 {
        let (_listener, client, _) = fixture();
        let mut grant = Grant::new(session(
            &client.origin,
            if mode == 1 { 100 } else { u64::MAX },
        ));
        let lease = grant.lease(&client.origin, 0).unwrap();
        if mode == 2 {
            lease.revoke();
        }
        let origin = if mode == 0 {
            "https://not-resolved.invalid"
        } else {
            &client.origin
        };
        let config = Config::from_lookup(origin, |key| match key {
            "OTTER_NATIVE_PROFILE_DIR" => Some(
                crate::test_temp_root()
                    .join("unused-profile")
                    .into_os_string(),
            ),
            // Intentionally incomplete CA configuration fails during preparation; authorization errors should precede Trust.
            "OTTER_LOOM_TLS_ORIGIN" => Some(origin.into()),
            _ => None,
        })
        .unwrap();
        let mut runtime = Runtime::start_configured(config, lease, target(), |_| {
            panic!("invalid credential consumed")
        })
        .unwrap();
        wait(runtime.wait_handle().unwrap());
        assert_eq!(
            runtime.try_join(),
            Some(if mode == 2 {
                Exit::Stopped
            } else {
                Exit::Preparation(PrepareError::Credential)
            })
        );
        assert!(runtime.try_ready().is_none());
    }
}
