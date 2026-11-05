//! 独立构造的协议/几何/时钟/代际 oracle；不由生产解析器生成预期值。
use crate::{
    tile_clock::Clock,
    tile_shared::{Frame, Key, Shared},
    tile_wire::*,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};
pub(crate) fn output() -> crate::tile_outputs::TileOutput {
    crate::tile_outputs::TileOutput {
        output_id: format!("monitor-{}", "a".repeat(64)),
        name: "fixture".into(),
        x: 0,
        y: 0,
        width: 2,
        height: 2,
    }
}
pub(crate) fn catalog(endpoint: &Endpoint, revision: u64) -> Value {
    json!({"protocolVersion":PROTOCOL,"revision":revision,"endpoints":[{"endpoint":endpoint,"online":true,"appliedRevision":null}],
        "layouts":[{"protocolVersion":PROTOCOL,"wallId":"wall-1","revision":revision,"bounds":{"x":0,"y":0,"width":2,"height":2},
        "tiles":[{"tileId":"tile-1","endpointId":endpoint.endpoint_id,"rect":{"x":0,"y":0,"width":2,"height":2},"rotation":"deg0"}],"placements":[]}],
        "timing":{"clockId":"boot-1","serverTimeMs":1000,"scenes":[{"wallId":"wall-1","revision":revision,"preparedAtMs":1000,"activateAtMs":1000}]}})
}
pub(crate) fn parsed(endpoint: &Endpoint) -> State {
    State::parse(&serde_json::to_vec(&catalog(endpoint, 2)).unwrap()).unwrap()
}
#[test]
fn wire_rejects_duplicate_ownership_overlap_outside_future_and_bad_crop() {
    let endpoint = Endpoint::image("device-1", &output());
    let original = catalog(&endpoint, 2);
    assert!(State::parse(&serde_json::to_vec(&original).unwrap()).is_ok());
    let mut cases = Vec::new();
    let mut value = original.clone();
    let duplicate = value["endpoints"][0].clone();
    value["endpoints"].as_array_mut().unwrap().push(duplicate);
    cases.push(value);
    let mut value = original.clone();
    let mut tile = value["layouts"][0]["tiles"][0].clone();
    tile["tileId"] = "other".into();
    tile["endpointId"] = "other-endpoint".into();
    value["layouts"][0]["tiles"]
        .as_array_mut()
        .unwrap()
        .push(tile);
    cases.push(value);
    let mut value = original.clone();
    value["layouts"][0]["tiles"][0]["rect"]["width"] = 3.into();
    cases.push(value);
    let mut value = original.clone();
    value["layouts"][0]["revision"] = 3.into();
    cases.push(value);
    let mut value = original.clone();
    value["layouts"][0]["bounds"]["x"] = 1_000_001.into();
    cases.push(value);
    let mut value = original.clone();
    value["timing"]["scenes"][0]["activateAtMs"] = 11_001.into();
    cases.push(value);
    let mut value = original.clone();
    value["endpoints"][0]["appliedRevision"] = 1.into();
    cases.push(value);
    let mut value = original.clone();
    value["endpoints"][0]
        .as_object_mut()
        .unwrap()
        .remove("appliedRevision");
    cases.push(value);
    let mut value = original;
    value["unexpected"] = true.into();
    cases.push(value);
    for value in cases {
        assert!(
            State::parse(&serde_json::to_vec(&value).unwrap()).is_err(),
            "{value}"
        );
    }
}
#[test]
fn endpoint_identity_is_exact_and_advertises_only_implemented_capabilities() {
    let a = Endpoint::image("device-1", &output());
    assert_eq!(a.render_modes, ["image", "raw_bgra", "surface_v1"]);
    assert_eq!(a.input_capabilities, ["pointer"]);
    assert_eq!(a.scheduled_presentation, Some(true));
    assert_eq!(a.display.as_ref().unwrap().name, "fixture");
    assert!(a.display.as_ref().unwrap().can_identify);
    assert_ne!(
        a.endpoint_id,
        Endpoint::image("device-2", &output()).endpoint_id
    );
    assert_eq!(
        a.endpoint_id,
        "tile-38af766fa51e2df52d643a5d8e498154517187d8bfc43f21a57b3ce1c495c0d5"
    );
}
#[test]
fn pixel_centers_rotation_and_crop_have_independent_corner_expectations() {
    let endpoint = Endpoint::image("device-1", &output());
    let state = parsed(&endpoint);
    let mut tile = state.layouts[0].tiles[0].clone();
    let p = Placement {
        placement_id: "p".into(),
        source: Source {
            kind: "image".into(),
            id: format!("sha256:{}", "a".repeat(64)),
        },
        rect: Rect {
            x: 1.,
            y: 0.,
            width: 2.,
            height: 2.,
        },
        source_crop: Rect {
            x: 0.25,
            y: 0.,
            width: 0.5,
            height: 1.,
        },
        z_index: 0,
        interactive: false,
    };
    let projection = crate::tile_geometry::project(&tile, &p).unwrap();
    assert_eq!(
        projection.crop,
        Rect {
            x: 0.25,
            y: 0.,
            width: 0.25,
            height: 1.
        }
    );
    assert_eq!(projection.quad, [[0.5, 0.], [1., 0.], [0.5, 1.]]);
    tile.rotation = Rotation::Right;
    assert_eq!(crate::tile_geometry::native(&tile, [0., 0.]), [0., 1.]);
    assert_eq!(
        crate::tile_geometry::rotate([0.25, 0.25], Rotation::Right),
        [0.75, 0.25]
    );
}
#[test]
fn clock_rejects_high_rtt_expires_and_changes_boot_without_early_scene() {
    let start = Instant::now();
    let mut clock = Clock::default();
    let mut t = Timing {
        clock_id: "boot".into(),
        server_time_ms: 1000,
        scenes: vec![],
    };
    assert!(!clock.observe(&t, start, start + Duration::from_millis(100)));
    assert!(!clock.permits(1100, start + Duration::from_millis(150)));
    assert!(!clock.permits(1100, start + Duration::from_millis(201)));
    assert!(clock.permits(1100, start + Duration::from_millis(202)));
    assert!(clock.estimate(start + Duration::from_secs(16)).is_none());
    t.clock_id = "new".into();
    assert!(clock.observe(&t, start, start + Duration::from_secs(2)));
    assert!(clock.estimate(start + Duration::from_secs(2)).is_none());
}
pub(crate) fn shared_frame() -> (Arc<Shared>, Frame) {
    let endpoint = Endpoint::image("device-1", &output());
    let state = parsed(&endpoint);
    let key = Key {
        endpoint: endpoint.endpoint_id,
        lease: "lease".into(),
        wall: "wall-1".into(),
        revision: 2,
        clock: "boot-1".into(),
    };
    let shared = Arc::new(Shared::default());
    {
        let mut s = shared.lock();
        s.desired = Some(key.clone());
        s.black = false;
        s.until = Some(Instant::now() + Duration::from_secs(6));
        s.clock.observe(
            state.timing.as_ref().unwrap(),
            Instant::now(),
            Instant::now(),
        );
    }
    (
        shared,
        Frame {
            key,
            layout: state.layouts[0].clone(),
            size: Size {
                width: 2,
                height: 2,
            },
            images: BTreeMap::new(),
            arts: BTreeMap::new(),
            live_until: None,
        },
    )
}
#[test]
fn late_frames_and_present_receipts_cannot_revive_stopped_or_replaced_session() {
    let (shared, frame) = shared_frame();
    let key = frame.key.clone();
    shared.publish(1, frame);
    assert!(shared.lock().visible(Instant::now()));
    assert!(shared.lock().presented.is_none());
    let rendered = shared.lock().frame.clone().unwrap();
    shared.confirm(&rendered, Instant::now());
    assert_eq!(shared.lock().presented.as_ref().unwrap().0, key);
    shared.lock().clear("replaced");
    shared.confirm(&rendered, Instant::now());
    assert!(shared.lock().presented.is_none());
    let (_, frame) = shared_frame();
    shared.publish(1, frame);
    assert!(shared.lock().frame.is_none());
    shared.stop();
    let (_, frame) = shared_frame();
    let generation = shared.lock().generation;
    shared.publish(generation, frame);
    assert!(shared.lock().frame.is_none());
}
#[test]
fn black_ack_requires_actual_window_receipt_and_auth_expiry_is_independent() {
    let (shared, frame) = shared_frame();
    shared.publish(1, frame);
    {
        let mut s = shared.lock();
        s.black = true;
        s.black_revision = Some(3);
        assert!(s.black_confirmed.is_none());
    }
    shared.confirm_black(Instant::now());
    assert_eq!(shared.lock().black_confirmed, Some(3));
    {
        let mut s = shared.lock();
        s.black = false;
        s.until = Some(Instant::now() - Duration::from_millis(1));
    }
    assert!(!shared.lock().visible(Instant::now()));
}
#[test]
fn cli_rejects_ambiguous_modes_relative_profile_and_untrusted_origin() {
    use crate::tile_cli::parse;
    use std::ffi::OsString;
    for args in [
        vec!["--tile"],
        vec!["--tile-outputs", "--loom", "https://localhost"],
        vec!["--tile", "--loom", "http://remote.invalid"],
        vec![
            "--tile",
            "--loom",
            "https://localhost",
            "--profile",
            "relative",
        ],
        vec!["--tile-output", "monitor-no", "--loom", "https://localhost"],
    ] {
        assert!(parse(&args.into_iter().map(OsString::from).collect::<Vec<_>>()).is_err());
    }
    assert!(parse(&["--tile-outputs".into()]).unwrap().is_some());
    assert!(parse(&["--smoke-test".into()]).unwrap().is_none());
}
