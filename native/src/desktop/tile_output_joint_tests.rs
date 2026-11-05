//! 实际Output owner换代；只替换本测试的枚举快照，不改Windows显示布局或用户设备。
use super::*;
use crate::desktop::tile_terminal::output::{Output, Status};
use crate::{
    pixels::Storage,
    tile_shared::{Key, Shared},
};
use std::sync::{Arc, Weak};

struct Generation {
    key: Key,
    shared: Weak<Shared>,
    pixels: Weak<Storage>,
}
impl Generation {
    fn reclaimed(&self) {
        assert!(
            self.shared.upgrade().is_none(),
            "old Runtime/window still owns Shared"
        );
        assert!(
            self.pixels.upgrade().is_none(),
            "old generation still owns Pixels"
        );
    }
}
fn presented(
    f: &Fixture,
    output: &mut Output,
    owner: &mut Owner,
    prior: Option<&Key>,
    name: &str,
    size: (u32, u32),
) -> Generation {
    loop {
        f.pump();
        assert_eq!(
            output.tick(owner, &f.origin, Some(&f.profile)).unwrap(),
            Status::Running
        );
        let runtime = output.runtime_for_test().unwrap();
        let ready = {
            let s = runtime.shared.lock();
            s.presented
                .as_ref()
                .is_some_and(|p| prior.is_none_or(|old| p.0.lease != old.lease))
        };
        if !ready {
            continue;
        }
        fixture::mark_pixels(f, runtime, name);
        let s = runtime.shared.lock();
        let frame = s.frame.as_ref().unwrap();
        assert_eq!((frame.size.width, frame.size.height), size);
        return Generation {
            key: frame.key.clone(),
            shared: Arc::downgrade(&runtime.shared),
            pixels: frame.images.values().next().unwrap().pixels.downgrade(),
        };
    }
}
pub(super) fn run() {
    let _com = Apartment::new();
    let f = Fixture::new();
    let mut owner = super::super::authorize(&f.origin, Some(&f.profile)).unwrap();
    let mut descriptor = crate::tile_tests::output();
    (descriptor.x, descriptor.y) = (-30000, -30000);
    let mut output = Output::new(&descriptor.output_id);
    output.observe(Ok(vec![descriptor.clone()]));
    let first = presented(
        &f,
        &mut output,
        &mut owner,
        None,
        "tile-native-running.json",
        (2, 2),
    );
    assert_eq!(first.key.revision, 2);
    f.phase("tile-resize");
    descriptor.width = 4;
    descriptor.x += 10;
    output.observe(Ok(vec![descriptor.clone()]));
    // 正常回收期间可以Draining，但不允许新旧Runtime同时存活。
    loop {
        f.pump();
        let status = output
            .tick(&mut owner, &f.origin, Some(&f.profile))
            .unwrap();
        if status == Status::Running {
            break;
        }
        assert_eq!(status, Status::Draining);
        assert!(output.runtime_for_test().is_none());
    }
    first.reclaimed();
    let resized = presented(
        &f,
        &mut output,
        &mut owner,
        Some(&first.key),
        "tile-native-resized.json",
        (4, 2),
    );
    assert_eq!(resized.key.endpoint, first.key.endpoint);
    assert_eq!(resized.key.revision, 3);
    f.phase("tile-remove");
    output.observe(Err("test output temporarily absent".into()));
    loop {
        f.pump();
        match output
            .tick(&mut owner, &f.origin, Some(&f.profile))
            .unwrap()
        {
            Status::Draining => assert!(output.runtime_for_test().is_none()),
            Status::Waiting => break,
            status => panic!("missing output status {status:?}"),
        }
    }
    resized.reclaimed();
    let mut wrong = descriptor.clone();
    wrong.output_id = format!("monitor-{}", "b".repeat(64));
    output.observe(Ok(vec![wrong]));
    for _ in 0..5 {
        f.pump();
        assert_eq!(
            output
                .tick(&mut owner, &f.origin, Some(&f.profile))
                .unwrap(),
            Status::Waiting
        );
        assert!(output.runtime_for_test().is_none());
    }
    f.mark(
        "tile-native-missing.json",
        json!({"oldGenerationReclaimed":true,"differentMonitorRejected":true}),
    );
    f.phase("tile-return");
    output.observe(Ok(vec![descriptor.clone()]));
    let returned = presented(
        &f,
        &mut output,
        &mut owner,
        Some(&resized.key),
        "tile-native-returned.json",
        (4, 2),
    );
    assert_eq!(returned.key.endpoint, first.key.endpoint);
    assert_eq!(returned.key.revision, 3);
    let revoked = std::env::var_os("OTTER_TILE_JOINT_REVOKE").is_some();
    if revoked {
        loop {
            f.pump();
            if output
                .tick(&mut owner, &f.origin, Some(&f.profile))
                .is_err()
            {
                break;
            }
        }
    } else {
        f.phase("tile-stop");
        output.stop();
    }
    output.observe(Ok(vec![descriptor]));
    while output
        .tick(&mut owner, &f.origin, Some(&f.profile))
        .unwrap()
        != Status::Stopped
    {
        f.pump();
    }
    returned.reclaimed();
    assert!(output.runtime_for_test().is_none());
    drop(output);
    owner.close();
    while !owner.finished() {
        owner.poll();
        f.pump();
    }
    f.mark("tile-output-complete.json", json!({"recovery":true,"revoked":revoked,"pixelsReleased":true,"threadsJoined":true,"sameEndpoint":true,"differentLeases":true,"stopSticky":true,"physicalHotplug":false,"ordinaryOutputExe":false}));
}
