//! 真实Loom识别命令→独立Runtime→自有离屏HWND→报告/关闭/过期/恢复；不改真实屏幕。
use super::*;
pub(super) fn run() {
    let _com = Apartment::new();
    let f = Fixture::new();
    let mut owner = super::super::authorize(&f.origin, Some(&f.profile)).unwrap();
    let lease = owner.lease(&f.origin, now_ms()).unwrap();
    let images = owner.lease(&f.origin, now_ms()).unwrap();
    let art = owner.lease(&f.origin, now_ms()).unwrap();
    let mut output = crate::tile_tests::output();
    (output.x, output.y, output.width, output.height) = (-30000, -30000, 640, 480);
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
    let (mut markers, mut dismissed, mut running, mut resumed) = (Vec::new(), false, false, false);
    loop {
        f.pump();
        window.tick().unwrap();
        if let Some(request) = window.test_identification_presented() {
            if !markers.contains(&request) {
                markers.push(request.clone());
                let s = runtime.shared.lock();
                assert!(s.identification.blocked() && s.presented.is_none());
                f.mark(&format!("tile-identify-marker-{}.json", markers.len()), json!({"requestId":request,"name":"fixture","width":640,"height":480,"nativeVisiblePaint":true,"layoutApplied":null}));
            }
        }
        if f.root.join("tile-identify-dismiss").exists() && !dismissed {
            assert_eq!(markers.len(), 1);
            window.test_identification_escape();
            assert!(window.alive());
            dismissed = true;
        }
        {
            let s = runtime.shared.lock();
            if !s.identification.blocked() && s.presented.is_some() {
                if !running {
                    drop(s);
                    fixture::mark_pixels(&f, &runtime, "tile-identify-running.json");
                    running = true;
                } else if markers.len() == 2 && !resumed {
                    drop(s);
                    fixture::mark_pixels(&f, &runtime, "tile-identify-resumed.json");
                    resumed = true;
                }
            }
        }
        if f.root.join("tile-stop").exists() {
            assert!(!revoked && dismissed && running && resumed && markers.len() == 2);
            runtime.request_stop();
        }
        if runtime.finished() {
            break;
        }
    }
    assert_eq!(markers.len(), if revoked { 1 } else { 2 });
    window.tick().unwrap();
    assert!(runtime
        .shared
        .lock()
        .identification
        .marker(Instant::now())
        .is_none());
    window.hide();
    drop(window);
    drop(runtime);
    owner.close();
    while !owner.finished() {
        owner.poll();
        f.pump();
    }
    f.mark("tile-identify-complete.json", json!({"revoked":revoked,"threadsJoined":true,"markerCleared":true,"dismissed":dismissed,"expiredAndResumed":resumed,"physicalOutput":false,"ordinaryOutputExe":false}));
}
