//! 同一个真实 child 的 Canvas→PNG→父侧像素、rAF、update、暂停与进程回收。
use super::*;
use crate::surface_viewport::Viewport;
use sha2::{Digest, Sha256};
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-canvas.js");
pub(crate) fn pixel(frame: &Frame, x: usize, y: usize) -> &[u8] {
    let image = frame.images.values().next().unwrap();
    let start = (y * image.width as usize + x) * 4;
    &image.pixels[start..start + 4]
}
fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert_eq!(pixel(&frame, 2, 2), [0, 255, 0, 255]);
    assert_eq!(pixel(&frame, 76, 2), [0, 0, 0, 0]);
    let frame = session.animation_frame(&|| false).unwrap();
    assert_eq!(pixel(&frame, 2, 2), [255, 0, 0, 255]);
    let frame = session.suspend(&|| false).unwrap();
    assert_eq!(pixel(&frame, 2, 2), [255, 0, 0, 255]);
    assert!(!frame.animation_pending && !frame.resize_observed);
    session.resume(&|| false).unwrap();
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(pixel(&frame, 2, 2), [0, 0, 255, 255]);
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_canvas_real_child_raster_pixels_raf_update_suspend_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = Session::mount_in(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(Viewport::new([200, 80], 96).unwrap()),
        &|| false,
    )
    .unwrap();
    exercise(session, frame);
}
#[test]
fn javascript_canvas_child_rejects_oversize_update_and_releases_process_capacity() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, frame) = Session::mount_in(
        fixture::binding(),
        "let c;NeuroSurface.define({mount({root}){c=document.createElement('canvas');c.width=8;c.height=8;c.getContext('2d').fillRect(0,0,8,8);root.append(c)},update(){c.width=4097}})".into(),
        fixture::snapshot(3, 42), fixture::resources(),
        Some(Viewport::new([200, 80], 96).unwrap()), &|| false,
    ).unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert_eq!(pixel(&frame, 0, 0), [0, 0, 0, 255]);
    assert!(session
        .update(fixture::snapshot(4, 43), fixture::resources(), &|| false)
        .is_err());
    assert!(session.child.is_none() && probe.exited());
    assert_eq!(CAPACITY.used(), 0);
    assert!(session
        .update(fixture::snapshot(5, 44), fixture::resources(), &|| false)
        .is_err());
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_canvas_actual_product_worker_runs_same_native_raster() {
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
        Some(Viewport::new([200, 80], 96).unwrap()),
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
