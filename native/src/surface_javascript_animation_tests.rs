//! 同预算真实 child 的帧回调、跨 viewport 读取、暂停/取消/终态回收及摘要绑定产品门。
use super::*;
use crate::surface_viewport::Viewport;
use sha2::{Digest, Sha256};
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-animation.js");
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
    assert!(initial.animation_pending);
    let first = session.animation_frame(&|| false).unwrap();
    assert!(first.animation_pending);
    assert_eq!(report(&first)["ticks"], 1);
    assert_eq!(report(&first)["width"], 40);
    assert!(!session.suspend(&|| false).unwrap().animation_pending);
    assert!(session.animation_frame(&|| false).is_err());
    assert!(session.resume(&|| false).unwrap().animation_pending);
    session
        .set_viewport(Viewport::new([600, 240], 96).unwrap())
        .unwrap();
    let second = session.animation_frame(&|| false).unwrap();
    assert!(!second.animation_pending);
    assert_eq!(report(&second)["ticks"], 2);
    assert_eq!(report(&second)["width"], 120);
    assert!(
        report(&second)["timestamp"].as_f64().unwrap()
            >= report(&first)["timestamp"].as_f64().unwrap()
    );
    assert!(
        session
            .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
            .unwrap()
            .animation_pending
    );
    assert!(
        !session
            .animation_frame(&|| false)
            .unwrap()
            .animation_pending
    );
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
    println!("native-javascript-animation: same-child=true callback-clock=true cancel=true next-frame=true viewport=true suspended=true closed=true peak-job-bytes={}", probe.peak_job_bytes().unwrap());
}
#[test]
fn javascript_animation_real_child_frames_cancel_pause_update_and_cleanup() {
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
fn javascript_animation_callback_failure_and_infinite_loop_reap_child_without_raising_budget() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for callback in ["throw Error('frame failed')", "while(true){}"] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{root.textContent='ready';requestAnimationFrame(()=>{{{callback}}});}}}});");
        let (mut session, _) = Session::mount_in(
            fixture::binding(),
            source,
            fixture::snapshot(3, 42),
            fixture::resources(),
            Some(Viewport::new([400, 240], 96).unwrap()),
            &|| false,
        )
        .unwrap();
        let probe = session.child.as_ref().unwrap().probe().unwrap();
        assert!(session.animation_frame(&|| false).is_err());
        assert!(probe.exited());
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
fn javascript_animation_no_viewport_refuses_dispatch_and_dispose_drops_queued_callbacks() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    assert!(frame.animation_pending);
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert!(session.animation_frame(&|| false).is_err());
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_animation_actual_product_worker_runs_same_frame_queue_and_clock() {
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
