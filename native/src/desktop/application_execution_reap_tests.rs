//! Real Application waiters and hidden HWNDs verify deadline continuation after notification, without registering global input.
use super::*;
use crate::desktop::{
    loom_execution_tasks::{
        reap_fixture::{Gate, WAIT},
        shader_tests::{id, output, SERIAL},
        Tasks,
    },
    loom_input_job::PreparedBatch,
    loom_result_state::Status,
    Apartment,
};

#[test]
fn application_reaps_current_and_retired_execution_after_last_notification() {
    let _serial = SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance = crate::desktop::workspace_window_fixture::register();
        let document =
            DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
        let mut app = Application::new(instance, document);
        // Both rounds share one host; do not carry WM_QUIT from destroying the previous round's final window into the next wait.
        for retired in [false, true] {
            let wake = app.execution_wake.callback();
            let gate = Gate::new(move || wake());
            app.execution_jobs = Tasks::with_notify(gate.notify.clone());
            let unit = app.documents[0].state.unit_id.clone();
            let request = app
                .executions
                .begin(unit.clone(), id("reap-request"))
                .unwrap();
            app.documents[0]
                .state
                .execution_results
                .borrow_mut()
                .begin(request.clone());
            app.execution_jobs
                .start_shader_test(
                    unit.clone(),
                    request.clone(),
                    PreparedBatch::empty_for_test(
                        unit.clone(),
                        &app.documents[0].state.canvas.borrow().editor,
                    ),
                    &app.executions,
                    output,
                )
                .unwrap();
            gate.noticed.recv_timeout(WAIT).unwrap();
            let mut message = MSG::default();
            assert!(
                PeekMessageW(&mut message, None, WM_APP + 92, WM_APP + 92, PM_REMOVE).as_bool()
            );
            assert_eq!(message.message, WM_APP + 92);
            if retired {
                assert!(app.executions.finish(&unit, &request));
                app.documents[0]
                    .state
                    .execution_results
                    .borrow_mut()
                    .cancel(&request);
            }
            app.pending_windows(false).unwrap();
            assert_eq!(
                app.execution_jobs.len(),
                1,
                "notifier still owns the thread tail"
            );
            assert!(app.execution_jobs.deadline().is_some());
            let pump = Pump::new();
            // The notification is already consumed; the real Application deadline must return, not a second notification from a helper thread.
            while pump.step(&mut app) != WM_NULL {}
            assert_eq!(app.execution_jobs.len(), 1);
            gate.release.send(()).unwrap();
            app.execution_jobs.test_wait_finished(0);
            while !app.execution_jobs.is_empty() {
                pump.step(&mut app);
            }
            assert!(app.execution_jobs.deadline().is_none());
            assert!(gate.noticed.try_recv().is_err());
            assert_eq!(
                app.documents[0].state.execution_results.borrow().status(),
                if retired {
                    Status::Cancelled
                } else {
                    Status::Completed
                }
            );
            assert!(app.executions.active(&unit).is_none());
        }
    }
}

#[test]
fn application_reaps_invalidated_display_after_last_notification() {
    let _serial = SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance = crate::desktop::workspace_window_fixture::register();
        let image = crate::desktop::image::Image {
            width: 1,
            height: 1,
            pixels: vec![10, 20, 30, 255].into(),
        };
        let document = DocumentWindow::create(
            instance,
            State::new(Some(image.clone()), false).unwrap(),
            false,
        )
        .unwrap();
        let mut app = Application::new(instance, document);
        let wake = app.execution_wake.callback();
        let gate = Gate::new(move || wake());
        let state = &app.documents[0].state;
        assert!(app
            .display_composition
            .borrow_mut()
            .request(
                &state.unit_id,
                &image,
                &state.canvas.borrow().editor,
                gate.notify.clone(),
            )
            .unwrap()
            .is_none());
        gate.noticed.recv_timeout(WAIT).unwrap();
        let mut message = MSG::default();
        assert!(PeekMessageW(&mut message, None, WM_APP + 92, WM_APP + 92, PM_REMOVE).as_bool());
        app.display_composition.borrow_mut().invalidate();
        app.pending_windows(false).unwrap();
        assert!(app.display_composition.borrow().is_busy());

        // The sole completion message was consumed; subsequent reaping must rely only on the real application deadline.
        let pump = Pump::new();
        while pump.step(&mut app) != WM_NULL {}
        assert!(app.display_composition.borrow().is_busy());
        gate.release.send(()).unwrap();
        while app.display_composition.borrow().is_busy() {
            pump.step(&mut app);
        }
        assert!(app.display_composition.borrow().deadline().is_none());
        assert!(app.documents[0].state.resolved_display.borrow().is_none());
        assert!(gate.noticed.try_recv().is_err());
    }
}
