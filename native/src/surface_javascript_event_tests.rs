//! 事件模型进入真实 child/native click/同文档更新；动作仍由原权威快照与父侧重新验证。
use super::*;
use sha2::{Digest, Sha256};
const SOURCES: [&str; 4] = [
    include_str!("../tests/fixtures/javascript-event-target.js"),
    include_str!("../tests/fixtures/javascript-event-mutation.js"),
    include_str!("../tests/fixtures/javascript-event-integrity.js"),
    include_str!("../tests/fixtures/javascript-batch-events.js"),
];
fn exercise(mut session: Session, frame: Frame, case: usize) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let root = frame.scene.root().id().to_owned();
    assert!(frame.events.is_empty());
    if case == 3 {
        let button = frame.scene.root().children()[1].children()[0]
            .id()
            .to_owned();
        assert_eq!(fixture::text(&frame.scene), "原生🙂:42:0");
        for round in 0..2 {
            let frame = session.click(&button, &|| false).unwrap();
            let quote = if round == 0 { 42 } else { 45 };
            assert_eq!(
                fixture::text(&frame.scene),
                format!("原生🙂:{quote}:{}", round + 1)
            );
            assert_eq!(frame.events.len(), 2);
            for (index, event) in frame.events.iter().enumerate() {
                let value: Value = serde_json::from_slice(event.bytes()).unwrap();
                assert_eq!(value["generation"], 7);
                assert_eq!(value["baseRevision"], round + 3);
                assert_eq!(value["attachmentId"], "attachment:javascript");
                assert_eq!(
                    value["payload"],
                    serde_json::json!({"count":round+1,"phase":index+1,"quote":quote})
                );
            }
            let quote = if round == 0 { 45 } else { 48 };
            let next = session
                .update(
                    fixture::snapshot(round + 4, quote),
                    fixture::resources(),
                    &|| false,
                )
                .unwrap();
            assert_eq!(next.scene.root().id(), root);
            assert_eq!(
                fixture::text(&next.scene),
                format!("原生🙂:{quote}:{}", round + 1)
            );
            assert!(next.events.is_empty());
        }
    } else {
        let expected = [
            "event-target-ready",
            "event-mutation-ready",
            "event-integrity-ready",
        ];
        assert_eq!(fixture::text(&frame.scene), expected[case]);
        if case == 2 {
            let next = session
                .click(frame.scene.root().children()[1].id(), &|| false)
                .unwrap();
            assert_eq!(fixture::text(&next.scene), "event-integrity-clicked");
            assert!(next.events.is_empty());
        }
        let next = session
            .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
            .unwrap();
        assert_eq!(next.scene.root().id(), root);
        assert_eq!(
            fixture::text(&next.scene),
            if case == 2 {
                "event-integrity-updated"
            } else {
                expected[case]
            }
        );
        assert!(next.events.is_empty());
    }
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    let peak = probe.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
    println!("native-javascript-event-target: case={case} same-child=true ordered=true reaped=true peak-job-bytes={peak}");
}
#[test]
fn javascript_event_target_real_child_models_native_click_update_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for (case, source) in SOURCES.iter().enumerate() {
        let (session, frame) = Session::mount(
            fixture::binding(),
            (*source).into(),
            fixture::snapshot(3, 42),
            fixture::resources(),
            &|| false,
        )
        .unwrap();
        exercise(session, frame, case);
    }
}
#[test]
fn javascript_event_target_callback_throw_or_cpu_budget_reaps_child_and_capacity() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for body in ["throw Error('event callback rejected')", "while(true){}"] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{const button=document.createElement('button');button.textContent='event';root.appendChild(button);button.addEventListener('click',()=>{{{body}}});}}}});");
        let (mut session, frame) = Session::mount(
            fixture::binding(),
            source,
            fixture::snapshot(3, 42),
            fixture::resources(),
            &|| false,
        )
        .unwrap();
        let probe = session.child.as_ref().unwrap().probe().unwrap();
        let button = frame.scene.root().children()[0].id();
        let start = Instant::now();
        assert!(session.click(button, &|| false).is_err());
        assert!(start.elapsed() < Duration::from_secs(4));
        assert!(probe.exited());
        assert!(session.click(button, &|| false).is_err());
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_event_target_actual_product_worker_uses_same_dispatch_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    for (case, source) in SOURCES.iter().enumerate() {
        let (session, frame) = geometry_tests::mount_product(
            &path,
            source,
            42,
            crate::surface_viewport::Viewport::new([320, 200], 96).unwrap(),
        );
        exercise(session, frame, case);
    }
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}
