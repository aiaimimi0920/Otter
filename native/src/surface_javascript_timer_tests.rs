//! 原预算真实 child：timer 持久性/暂停/几何/更新、终态回收及摘要绑定实际产品。
use super::*;
use crate::surface_viewport::Viewport;
use sha2::{Digest, Sha256};
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-timers.js");
pub(crate) fn report(frame: &Frame) -> Value {
    serde_json::from_str(
        frame.scene.root().children()[1].props()["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}
fn exercise(mut session: Session, initial: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert!(initial.timer_delay_ms.is_some() && !initial.animation_pending);
    let first = session.timer_tick(&|| false).unwrap();
    assert_eq!(report(&first)["count"], 1);
    assert_eq!(report(&first)["width"], 40);
    assert!(first.timer_delay_ms.is_some());
    assert!(session.suspend(&|| false).unwrap().timer_delay_ms.is_none());
    assert!(session.timer_tick(&|| false).is_err());
    assert!(session.resume(&|| false).unwrap().timer_delay_ms.is_some());
    session
        .set_viewport(Viewport::new([600, 240], 96).unwrap())
        .unwrap();
    let second = session.timer_tick(&|| false).unwrap();
    assert_eq!(report(&second)["count"], 2);
    assert_eq!(report(&second)["width"], 120);
    assert!(report(&second)["time"].as_f64().unwrap() >= report(&first)["time"].as_f64().unwrap());
    assert!(second.timer_delay_ms.is_none() && !second.animation_pending);
    let updated = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert!(updated.timer_delay_ms.is_some());
    let updated = session.timer_tick(&|| false).unwrap();
    assert_eq!(report(&updated)["quote"], 45);
    assert!(updated.timer_delay_ms.is_none());
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
    println!("native-javascript-timers: same-child=true args=true cancel=true viewport=true suspend=true update=true closed=true peak-job-bytes={}", probe.peak_job_bytes().unwrap());
}
#[test]
fn javascript_timer_real_child_callbacks_pause_cancel_geometry_and_update() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = Session::mount_in(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(Viewport::new([400, 240], 96).unwrap()),
        &|| false,
    )
    .unwrap();
    exercise(session, frame);
}
#[test]
fn javascript_timer_callback_throw_loop_microtask_loop_reaps_under_original_budget() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for callback in [
        "throw Error('timer failed')",
        "while(true){}",
        "const f=()=>Promise.resolve().then(f);f()",
    ] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{root.textContent='ready';setTimeout(()=>{{{callback}}},0);}}}});");
        let (mut session, _) = Session::mount(
            fixture::binding(),
            source,
            fixture::snapshot(3, 42),
            fixture::resources(),
            &|| false,
        )
        .unwrap();
        let probe = session.child.as_ref().unwrap().probe().unwrap();
        assert!(session.timer_tick(&|| false).is_err());
        assert!(probe.exited());
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
fn javascript_timer_dispose_drops_pending_and_requires_no_viewport_without_layout_reads() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = "NeuroSurface.define({mount({root}){root.textContent='ready';setTimeout(()=>root.textContent='done',0);setInterval(()=>{throw Error('disposed')},2147483647)}});";
    let (mut session, _) = Session::mount(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let frame = session.timer_tick(&|| false).unwrap();
    assert_eq!(fixture::text(&frame.scene), "done");
    assert!(frame.viewport.is_none() && frame.timer_delay_ms.is_some());
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_timer_actual_product_worker_runs_same_timer_heap_clock_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    let (session, frame) = Session::mount_with_viewport(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(Viewport::new([400, 240], 96).unwrap()),
        &|| false,
        |endpoint, deadline| {
            worker::start_at(
                endpoint,
                deadline,
                &|| false,
                worker::Policy {
                    suffix: " --native-javascript-worker",
                    maximum: CHILD_LIMIT,
                    pool: &CAPACITY,
                    environment: worker::Environment::SystemRoot,
                },
                &path,
            )
        },
    )
    .unwrap();
    exercise(session, frame);
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}
