//! 持久 child 的 observer→rAF、隐藏/移除/重接、暂停恢复与终态清理；产品 EXE 独立摘要门。
use super::*;
use crate::surface_viewport::Viewport;
use sha2::{Digest, Sha256};
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-resize.js");
pub(crate) fn report(frame: &Frame) -> Value {
    serde_json::from_str(
        frame.scene.root().children().last().unwrap().props()["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}
fn exercise(mut session: Session, initial: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert_eq!(report(&initial)["count"], 1);
    assert_eq!(report(&initial)["frames"], 0);
    assert_eq!(report(&initial)["entry"]["content"], 188);
    assert!(initial.resize_observed && initial.animation_pending);
    let frame = session.animation_frame(&|| false).unwrap();
    assert_eq!(report(&frame)["bounds"], 200);
    assert!(!frame.animation_pending);
    session
        .set_viewport(Viewport::new([1000, 480], 192).unwrap())
        .unwrap();
    let frame = session.animation_frame(&|| false).unwrap();
    assert_eq!(report(&frame)["count"], 2);
    assert_eq!(report(&frame)["frames"], 1);
    assert_eq!(report(&frame)["entry"]["content"], 238);
    assert!(frame.animation_pending);
    let frame = session.animation_frame(&|| false).unwrap();
    assert_eq!(report(&frame)["bounds"], 250);
    assert_eq!(report(&frame)["frames"], 2);
    for mode in 1..=5 {
        let frame = session
            .update(
                fixture::snapshot(3 + mode, mode as i32),
                fixture::resources(),
                &|| false,
            )
            .unwrap();
        let expected = if mode % 2 == 1 { 0 } else { 250 };
        if mode < 5 {
            assert_eq!(report(&frame)["entry"]["border"], expected);
            assert!(frame.animation_pending);
            session.animation_frame(&|| false).unwrap();
        } else {
            assert!(!frame.resize_observed && !frame.animation_pending);
        }
    }
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
    println!("native-javascript-resize: same-child=true initial=true resize-to-raf=true content-box=true hidden-detach=true disconnected=true closed=true");
}
#[test]
fn javascript_resize_real_child_native_boxes_frames_visibility_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = Session::mount_in(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(Viewport::new([800, 480], 192).unwrap()),
        &|| false,
    )
    .unwrap();
    exercise(session, frame);
}
#[test]
fn javascript_resize_callback_failure_and_loop_reap_same_child() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for callback in [
        "throw Error('callback')",
        "while(true){}",
        "target.style.width='150px'",
    ] {
        let source=format!("let target,ro;NeuroSurface.define({{mount({{root}}){{root.innerHTML='<div></div>';target=root.firstElementChild;target.style.cssText='box-sizing:border-box;width:100px;height:40px'}},update(){{ro=new ResizeObserver(()=>{{{callback}}});ro.observe(target)}}}});");
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
        assert!(session
            .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
            .is_err());
        assert!(probe.exited());
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_resize_actual_product_worker_runs_same_native_observer() {
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
        Some(Viewport::new([800, 480], 192).unwrap()),
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
