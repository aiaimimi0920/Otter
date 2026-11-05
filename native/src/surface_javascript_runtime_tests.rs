//! 显式运行封存产品 EXE 的真正私有 worker；输入摘要由外部调用者提供，不信任 target 缓存。
use super::*;
use sha2::{Digest, Sha256};
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_actual_product_worker_preserves_state_and_reaps() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(
        std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").expect("sealed product path"),
    );
    let expected =
        std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").expect("external expected SHA-256");
    assert!(path.is_absolute());
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), expected);
    let (mut session, frame) = Session::mount_with(
        fixture::binding(),
        fixture::SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
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
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let target = frame.scene.root().children()[1].id().to_owned();
    assert_eq!(fixture::text(&frame.scene), "原生🙂:42:0");
    let frame = session.click(&target, &|| false).unwrap();
    assert_eq!(fixture::text(&frame.scene), "原生🙂:42:1");
    assert_eq!(frame.events.len(), 1);
    session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    let frame = session.click(&target, &|| false).unwrap();
    assert_eq!(fixture::text(&frame.scene), "原生🙂:45:2");
    assert_eq!(frame.events[0].revision(), 4);
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    let peak = probe.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= CHILD_LIMIT);
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    println!("native-javascript-product-worker: preserved=true async=true updates=true emits=true dispose=true reaped=true peak-job-bytes={peak} product-ui=false loom=false");
}
