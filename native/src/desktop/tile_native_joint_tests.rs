//! 独立原生测试进程沿真实管理器菜单/输出授权/Runtime/窗口；物理输出和普通EXE仍另验。
use super::*;
use serde_json::json;
#[path = "tile_native_joint_fixture.rs"]
mod fixture;
use fixture::{Apartment, Fixture};
#[path = "tile_surface_local_joint_tests.rs"]
mod local;
#[path = "tile_surface_text_joint_tests.rs"]
mod text;
#[test]
#[ignore = "explicit real Loom LAN Tile handlerless values and text-only form terminal gate"]
fn tile_native_joint_local_values_and_cleanup() {
    local::run();
}
#[test]
#[ignore = "explicit real Loom LAN Tile native text/textarea/number and Art terminal gate"]
fn tile_native_joint_text_and_cleanup() {
    text::run();
}
#[path = "tile_surface_action_joint_tests.rs"]
mod actions;
#[path = "tile_surface_slider_joint_tests.rs"]
mod slider;
#[path = "tile_surface_value_joint_tests.rs"]
mod values;
#[test]
#[ignore = "explicit real Loom LAN Tile Slider input/change and real Art terminal gate"]
fn tile_native_joint_slider_and_cleanup() {
    slider::run();
}
#[test]
#[ignore = "explicit real Loom LAN Tile switch/select and real Art terminal business gate"]
fn tile_native_joint_values_and_cleanup() {
    values::run();
}
#[path = "tile_surface_joint_tests.rs"]
mod art;
#[test]
#[ignore = "explicit real Loom LAN Tile actions and terminal business result child"]
fn tile_native_joint_actions_and_cleanup() {
    actions::run();
}
#[path = "tile_identification_joint_tests.rs"]
mod identification;
#[path = "tile_live_joint_tests.rs"]
mod live;
#[path = "tile_mixed_joint_tests.rs"]
mod mixed;
#[test]
#[ignore = "explicit real Loom LAN image Live Art dynamic mixed lifecycle child"]
fn tile_native_joint_mixed_and_cleanup() {
    mixed::run();
}
#[path = "tile_output_joint_tests.rs"]
mod recovery;
#[test]
#[ignore = "explicit real Loom LAN Tile identification, dismissal, expiry and cleanup child"]
fn tile_native_joint_identification_and_cleanup() {
    identification::run();
}
#[test]
#[ignore = "explicit real Loom LAN Tile Art display and view cleanup child"]
fn tile_native_joint_art_and_cleanup() {
    art::run();
}
#[test]
#[ignore = "explicit real Loom LAN mixed image/Live, PNG and cleanup child"]
fn tile_native_joint_live_and_cleanup() {
    live::run();
}

#[test]
#[ignore = "explicit real Loom LAN Tile output recovery and generation cleanup child"]
fn tile_native_joint_output_recovery_and_cleanup() {
    recovery::run();
}

#[test]
#[ignore = "explicit real Loom LAN Tile manager pairing and signed renewal child"]
fn tile_native_joint_manager_pair_approve_and_renew() {
    let _com = Apartment::new();
    let f = Fixture::new();
    assert!(
        !f.profile.exists(),
        "first pairing must create a fresh isolated identity"
    );
    let window = unsafe { create(&[crate::tile_tests::output()]).unwrap() };
    let mut owner = Owner::new();
    let Event::Pairing { device, .. } = f.menu(&window, &mut owner, 1) else {
        panic!("pairing event")
    };
    f.mark("tile-pairing.json", json!({"deviceId":device}));
    f.phase("tile-approved");
    let Event::Authorized {
        device: approved, ..
    } = f.menu(&window, &mut owner, 2)
    else {
        panic!("approval event")
    };
    assert_eq!(device, approved);
    let Event::Authorized {
        device: renewed, ..
    } = f.menu(&window, &mut owner, 3)
    else {
        panic!("renewal event")
    };
    assert_eq!(device, renewed);
    assert!(owner.lease(&f.origin, now_ms()).is_ok());
    owner.close();
    while !owner.finished() {
        owner.poll();
        f.pump();
    }
    drop(window);
    f.mark("tile-manager-complete.json", json!({"deviceId":device,"pairing":true,"approved":true,"signedRenewal":true,"managerClosed":true}));
}

#[test]
#[ignore = "explicit real Loom LAN Tile output, native present and cleanup child"]
fn tile_native_joint_output_present_and_cleanup() {
    let _com = Apartment::new();
    let f = Fixture::new();
    assert!(
        f.profile.is_dir(),
        "independent output reuses only its test profile"
    );
    let mut owner = super::authorize(&f.origin, Some(&f.profile)).unwrap();
    let lease = owner.lease(&f.origin, now_ms()).unwrap();
    let images = owner.lease(&f.origin, now_ms()).unwrap();
    let art = owner.lease(&f.origin, now_ms()).unwrap();
    let mut output = crate::tile_tests::output();
    output.x = -30000;
    output.y = -30000;
    let runtime = Runtime::start(
        config(&f.origin, Some(&f.profile)).unwrap(),
        lease,
        images,
        art,
        output.clone(),
    )
    .unwrap();
    let window = crate::desktop::tile_test_window(runtime.shared.clone(), &output);
    let revoked = std::env::var_os("OTTER_TILE_JOINT_REVOKE").is_some();
    let mut first = false;
    let mut frozen = false;
    let mut black = false;
    let mut resumed = false;
    let mut pixels = Vec::new();
    loop {
        f.pump();
        window.tick().unwrap();
        {
            let s = runtime.shared.lock();
            if let Some(frame) = &s.frame {
                if !pixels
                    .iter()
                    .any(|p: &std::sync::Weak<crate::pixels::Storage>| {
                        std::sync::Weak::ptr_eq(
                            p,
                            &frame.images.values().next().unwrap().pixels.downgrade(),
                        )
                    })
                {
                    pixels.push(frame.images.values().next().unwrap().pixels.downgrade());
                }
            }
            if s.presented.is_some() && !s.frozen && !s.black && !first {
                drop(s);
                fixture::mark_pixels(&f, &runtime, "tile-native-running.json");
                first = true;
            } else if s.frozen && s.presented.is_some() && !frozen {
                assert!(first && !s.black);
                drop(s);
                fixture::mark_pixels(&f, &runtime, "tile-native-frozen.json");
                frozen = true;
            } else if s.black_revision.is_some() && s.black_confirmed == s.black_revision && !black
            {
                assert!(frozen && s.frame.is_none() && s.presented.is_none());
                f.mark(
                    "tile-native-black.json",
                    json!({"revision":s.black_confirmed,"nativeBlackPresentConfirmed":true}),
                );
                black = true;
            } else if black && s.presented.is_some() && !s.black && !resumed {
                drop(s);
                fixture::mark_pixels(&f, &runtime, "tile-native-resumed.json");
                resumed = true;
            }
        }
        if f.root.join("tile-stop").exists() {
            assert!(!revoked && resumed);
            runtime.request_stop();
        }
        if runtime.finished() {
            break;
        }
    }
    assert!(first);
    if revoked {
        assert!(runtime.shared.lock().stopped);
    } else {
        assert!(frozen && black && resumed);
    }
    window.tick().unwrap();
    assert!(runtime.shared.lock().frame.is_none());
    assert!(runtime.shared.lock().presented.is_none());
    window.hide();
    drop(window);
    drop(runtime);
    owner.close();
    while !owner.finished() {
        owner.poll();
        f.pump();
    }
    drop(owner);
    assert!(!pixels.is_empty() && pixels.iter().all(|p| p.upgrade().is_none()));
    f.mark("tile-output-complete.json", json!({"revoked":revoked,"pixelsReleased":true,"threadsJoined":true,"physicalCrossDevice":false,"ordinaryOutputExe":false}));
}
