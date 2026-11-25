//! Deterministic notification-tail regressions sharing production runners/permits/reclamation; not evidence of actual GPU rendering.
use super::{
    reap_fixture::{Gate, WAIT},
    shader_tests::{id, output, SERIAL},
    *,
};
use crate::desktop::{loom_input_job::PreparedBatch, shader_execution_job as shader};
use std::sync::{mpsc, Arc};

#[test]
fn notified_shader_reaps_current_and_retired_tasks_without_second_notice() {
    let _serial = SERIAL.lock().unwrap();
    for reason in ["current", "closed", "replaced", "edited"] {
        let gate = Gate::new(|| {});
        let mut tasks = Tasks::with_notify(gate.notify.clone());
        let mut registry = Requests::default();
        let request = registry.begin(id("local"), id("request")).unwrap();
        let (run, waiting) = mpsc::channel();
        assert!(tasks.deadline().is_none());
        tasks
            .start_shader_test(
                id("local"),
                request,
                PreparedBatch::empty_for_test(id("local"), &crate::editor::Editor::default()),
                &registry,
                move |cancel| {
                    waiting.recv_timeout(WAIT).unwrap();
                    output(cancel)
                },
            )
            .unwrap();
        assert!(tasks.deadline().is_none());
        tasks.poll();
        assert!(tasks.take_settled(&registry).is_none());
        run.send(()).unwrap();
        gate.noticed.recv_timeout(WAIT).unwrap();
        match reason {
            "closed" => tasks.retain_units(|_| false),
            "replaced" => {
                registry.begin(id("local"), id("replacement")).unwrap();
                tasks.retire_stale(&registry);
            }
            "edited" => tasks.retire_changed(|_, _| false),
            _ => {}
        }
        assert!(tasks.deadline().is_some());
        tasks.poll();
        assert!(tasks.take_settled(&registry).is_none());
        assert_eq!(tasks.len(), 1);
        gate.release.send(()).unwrap();
        tasks.test_wait_finished(0);
        tasks.poll();
        assert!(
            tasks.deadline().is_none(),
            "unconsumed result has no worker"
        );
        assert_eq!(tasks.len(), 1);
        let report = tasks.take_settled(&registry).unwrap();
        assert_eq!(report.current, reason == "current");
        if reason == "current" {
            assert!(matches!(report.completion, Completion::Shader(Ok(_))));
        } else {
            assert!(matches!(
                report.completion,
                Completion::Shader(Err(shader::Error::Cancelled))
            ));
        }
        assert!(tasks.is_empty());
        assert!(tasks.take_completed(&registry).is_none());
        assert!(gate.noticed.try_recv().is_err());
    }
}

#[test]
fn shader_worker_panic_is_not_masked_by_cancel_during_notification() {
    let _serial = SERIAL.lock().unwrap();
    let gate = Gate::new(|| {});
    let mut tasks = Tasks::with_notify(gate.notify.clone());
    let mut registry = Requests::default();
    let request = registry.begin(id("local"), id("request")).unwrap();
    tasks
        .start_shader_test(
            id("local"),
            request,
            PreparedBatch::empty_for_test(id("local"), &crate::editor::Editor::default()),
            &registry,
            |_| panic!("injected execution panic"),
        )
        .unwrap();
    gate.noticed.recv_timeout(WAIT).unwrap();
    tasks.retain_units(|_| false);
    tasks.poll();
    assert!(tasks.take_settled(&registry).is_none());
    assert!(tasks.deadline().is_some());
    gate.release.send(()).unwrap();
    tasks.test_wait_finished(0);
    tasks.poll();
    let report = tasks.take_settled(&registry).unwrap();
    assert!(matches!(
        report.completion,
        Completion::Shader(Err(shader::Error::WorkerFailed))
    ));
    assert!(!report.current);
    assert!(tasks.deadline().is_none());
}

#[test]
fn shader_notifier_panic_preserves_success_and_reaps_once() {
    let _serial = SERIAL.lock().unwrap();
    let mut tasks = Tasks::with_notify(Arc::new(|| panic!("injected notifier panic")));
    let mut registry = Requests::default();
    let request = registry.begin(id("local"), id("request")).unwrap();
    tasks
        .start_shader_test(
            id("local"),
            request,
            PreparedBatch::empty_for_test(id("local"), &crate::editor::Editor::default()),
            &registry,
            output,
        )
        .unwrap();
    tasks.test_wait_finished(0);
    assert!(tasks.deadline().is_some());
    tasks.poll();
    assert!(matches!(
        tasks.take_settled(&registry).unwrap().completion,
        Completion::Shader(Ok(_))
    ));
    assert!(tasks.deadline().is_none());
    assert!(tasks.take_completed(&registry).is_none());
}
