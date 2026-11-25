//! Real WM_CLOSE and application waiting with an intentionally unfinished edit.
use super::*;
use std::sync::mpsc;

#[test]
fn closing_window_retires_decode_for_both_pin_and_in_place_replacement() {
    let _apartment = desktop::Apartment::new().unwrap();
    unsafe {
        let instance = desktop::workspace_window_fixture::register();
        let first =
            DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
        let mut app = Application::new(instance, first);
        // Keep one host across both cases so destroying a fixture does not queue WM_QUIT.
        for new_pin in [false, true] {
            app.add_window(false).unwrap();
            let state = &app.documents[1].state;
            let old_unit = state.unit_id.clone();
            let stamp = app.documents[0]
                .state
                .canvas
                .borrow()
                .editor
                .document_stamp();
            let (release, gate) = mpsc::channel();
            let cancelled = desktop::open_job::install_gated(state, new_pin, gate);

            SendMessageW(app.documents[1].window.0, WM_CLOSE, None, None);
            assert!(!app.documents[1].is_alive());
            assert!(cancelled.load(std::sync::atomic::Ordering::Acquire));
            app.collect_closed();
            assert_eq!(app.documents.len(), 1);
            assert!(!app.workspace.contains(&old_unit));
            assert_eq!(app.retired_edits.opens.len(), 1);

            let pump = super::super::execution_test_pump::Pump::new();
            while pump.step(&mut app) != WM_NULL {}
            assert_eq!(app.retired_edits.opens.len(), 1);
            release.send(()).unwrap();
            while app.retired_edits.deadline().is_some() {
                pump.step(&mut app);
            }
            assert!(app.error.is_none());
            assert_eq!(app.documents.len(), 1);
            let survivor = &app.documents[0].state;
            assert!(survivor.new_pin.borrow().is_none());
            assert!(stamp.matches(&survivor.canvas.borrow().editor));
        }
    }
}

#[test]
fn closed_window_releases_its_state_without_joining_and_pump_reaps_the_worker() {
    let _apartment = desktop::Apartment::new().unwrap();
    unsafe {
        let instance = desktop::workspace_window_fixture::register();
        let first =
            DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
        let mut app = Application::new(instance, first);
        app.add_window(false).unwrap();
        let closed = app.documents[0].window.0;
        let old_unit = app.documents[0].state.unit_id.clone();
        let survivor = app.documents[1].state.unit_id.clone();
        let stamp = app.documents[1]
            .state
            .canvas
            .borrow()
            .editor
            .document_stamp();
        let (release, waiting) = mpsc::channel();
        let (returning, returned) = mpsc::channel();
        *app.documents[0].state.crop_job.borrow_mut() =
            Some(desktop::crop_apply::gated_job(waiting, returning));

        SendMessageW(closed, WM_CLOSE, None, None);
        assert!(!app.documents[0].is_alive());
        app.collect_closed();
        assert_eq!(app.documents.len(), 1);
        assert!(!app.workspace.contains(&old_unit));
        assert!(app.retired_edits.deadline().is_some());
        assert!(matches!(
            returned.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));

        let pump = super::super::execution_test_pump::Pump::new();
        while pump.step(&mut app) != WM_NULL {}
        assert!(app.retired_edits.deadline().is_some());

        release.send(()).unwrap();
        assert!(returned.recv_timeout(Duration::from_secs(5)).unwrap());
        while app.retired_edits.deadline().is_some() {
            pump.step(&mut app);
        }
        assert!(app.error.is_none());
        assert_eq!(app.documents[0].state.unit_id, survivor);
        assert!(stamp.matches(&app.documents[0].state.canvas.borrow().editor));
    }
}

#[test]
fn each_remaining_raster_worker_retires_from_a_closed_hwnd_without_blocking() {
    use crate::{
        blur::Region, edit_worker_budget::test_gate::Gate, erase_layers::Layers, pixels::Pixels,
    };
    let _apartment = desktop::Apartment::new().unwrap();
    unsafe {
        let instance = desktop::workspace_window_fixture::register();
        let first =
            DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
        let mut app = Application::new(instance, first);
        let survivor = app.documents[0].state.unit_id.clone();
        let stamp = app.documents[0]
            .state
            .canvas
            .borrow()
            .editor
            .document_stamp();
        for kind in ["blur", "erase", "flip", "resize", "rasterize"] {
            app.add_window(false).unwrap();
            let doc = &app.documents[1];
            let old_unit = doc.state.unit_id.clone();
            let (gate, release, observed) = Gate::pair();
            let mut pending_pixels = None;
            let mut running_pixels = None;
            let layers = |base| Layers {
                size: (1, 1),
                base,
                annotations: None,
                annotation_size: None,
            };
            let revision = doc.state.canvas.borrow().editor.revision();
            match kind {
                "blur" => {
                    let request = |base| blur_job::Request {
                        stamp: doc.state.canvas.borrow().editor.document_stamp(),
                        target: None,
                        annotation_frame: None,
                        base_projection: None,
                        source: layers(base),
                        size: (1, 1),
                        area: Region {
                            x: 0,
                            y: 0,
                            width: 1,
                            height: 1,
                        },
                        sigma: 1.0,
                        revision,
                    };
                    let base = Pixels::from(vec![255; 4]);
                    running_pixels = Some(base.downgrade());
                    let mut job = blur_job::Job::gated(request(base), gate);
                    let pending = Pixels::from(vec![127; 4]);
                    pending_pixels = Some(pending.downgrade());
                    job.request(request(pending)).unwrap();
                    *doc.state.blur_job.borrow_mut() = job;
                }
                "erase" => {
                    let base = Pixels::from(vec![255; 4]);
                    running_pixels = Some(base.downgrade());
                    *doc.state.erase_job.borrow_mut() =
                        erase_job::Job::gated(layers(base), revision, gate);
                }
                "flip" => *doc.state.flip_job.borrow_mut() = flip_job::Job::gated(gate),
                "resize" => *doc.state.resize_job.borrow_mut() = resize_job::Job::gated(gate),
                "rasterize" => {
                    let base = Pixels::from(vec![255; 4]);
                    running_pixels = Some(base.downgrade());
                    *doc.state.rasterize_job.borrow_mut() = Some(
                        desktop::rasterize_input::Job::gated(layers(base), revision, gate),
                    );
                }
                _ => unreachable!(),
            }
            SendMessageW(doc.window.0, WM_CLOSE, None, None);
            assert!(!doc.is_alive(), "{kind}");
            app.collect_closed();
            assert_eq!(app.documents.len(), 1, "{kind}");
            assert!(!app.workspace.contains(&old_unit), "{kind}");
            assert_eq!(app.retired_edits.raster.len(), 1, "{kind}");
            assert!(
                pending_pixels.is_none_or(|pixels| pixels.upgrade().is_none()),
                "{kind}"
            );
            assert!(
                running_pixels
                    .as_ref()
                    .is_none_or(|pixels| pixels.upgrade().is_some()),
                "{kind}"
            );
            assert!(
                matches!(observed.try_recv(), Err(mpsc::TryRecvError::Empty)),
                "{kind}"
            );
            let pump = super::super::execution_test_pump::Pump::new();
            while pump.step(&mut app) != WM_NULL {}
            assert!(app.retired_edits.deadline().is_some(), "{kind}");
            release.send(()).unwrap();
            assert!(
                observed.recv_timeout(Duration::from_secs(5)).unwrap(),
                "{kind}"
            );
            while app.retired_edits.deadline().is_some() {
                pump.step(&mut app);
            }
            assert!(
                running_pixels.is_none_or(|pixels| pixels.upgrade().is_none()),
                "{kind}"
            );
            assert!(app.error.is_none(), "{kind}");
            let state = &app.documents[0].state;
            assert_eq!(state.unit_id, survivor, "{kind}");
            assert!(stamp.matches(&state.canvas.borrow().editor), "{kind}");
        }
    }
}

#[test]
fn closing_window_revokes_copy_and_drag_but_finishes_admitted_file_export() {
    let _apartment = desktop::Apartment::new().unwrap();
    let scratch = crate::test_temp_root();
    let id = desktop::document_state::new_unit_id().unwrap();
    let path = scratch.join(format!("otter-retired-export-{}.txt", id.as_str()));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _cleanup = Cleanup(path.clone());
    unsafe {
        let instance = desktop::workspace_window_fixture::register();
        let first =
            DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
        let mut app = Application::new(instance, first);
        app.add_window(false).unwrap();
        let doc = &app.documents[0];
        let (release_copy, copy_gate) = mpsc::channel();
        let (release_drag, drag_gate) = mpsc::channel();
        let (release_export, export_gate) = mpsc::channel();
        let (copy_done, copied) = mpsc::channel();
        let (drag_done, dragged) = mpsc::channel();
        desktop::copy_job::install_gated(&doc.state, copy_gate, copy_done);
        desktop::drag_input::install_gated(&doc.state, drag_gate, drag_done);
        let target = path.clone();
        *doc.state.export.borrow_mut() = Some(
            desktop::export_job::ExportJob::start(move || {
                use std::io::Write;
                export_gate
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(|e| e.to_string())?;
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(target)
                    .and_then(|mut file| file.write_all(b"admitted export"))
                    .map_err(|e| e.to_string())
            })
            .unwrap(),
        );
        SendMessageW(doc.window.0, WM_CLOSE, None, None);
        app.collect_closed();
        assert_eq!(app.documents.len(), 1);
        assert!(app.retired_edits.deadline().is_some());
        assert!(!path.exists());
        assert!(matches!(copied.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert!(matches!(dragged.try_recv(), Err(mpsc::TryRecvError::Empty)));

        let pump = super::super::execution_test_pump::Pump::new();
        while pump.step(&mut app) != WM_NULL {}
        release_copy.send(()).unwrap();
        release_drag.send(()).unwrap();
        release_export.send(()).unwrap();
        assert!(copied.recv_timeout(Duration::from_secs(5)).unwrap());
        assert!(dragged.recv_timeout(Duration::from_secs(5)).unwrap());
        while app.retired_edits.deadline().is_some() {
            pump.step(&mut app);
        }
        assert_eq!(std::fs::read(path).unwrap(), b"admitted export");
        assert!(app.error.is_none());
    }
}
