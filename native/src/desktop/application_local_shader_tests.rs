//! 同一隐藏应用主循环消费本地 Shader，验证来源/文档/配置/拓扑/取消/请求替换；不发新远端执行。
use super::*;
use crate::desktop::{
    loom_execution_tasks::shader_tests,
    loom_input_job::PreparedBatch,
    loom_input_stamp::SourceStamp,
    loom_result_state::{Results, Status},
    shader_image::{self, tests as oracle},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub(super) fn verify(app: &mut Application, real: bool) {
    let _serial = shader_tests::SERIAL.lock().unwrap();
    let unit = app.documents[0].state.unit_id.clone();
    let source = app.documents[1].state.unit_id.clone();
    let original = std::mem::replace(
        &mut *app.documents[0].state.execution_results.borrow_mut(),
        Results::new(unit.clone()),
    );
    let image = original.image_source("output_image").unwrap().image;
    let binding = app.documents[0]
        .state
        .execution_configuration
        .borrow_mut()
        .take();
    let original_workspace = app.workspace.clone();
    let original_accept = app.documents[0].state.accept_upstream.get();
    let execution = app.documents[0]
        .state
        .selected_art
        .borrow()
        .as_ref()
        .unwrap()
        .execution;
    // 原链测试已证明传播；这里禁止新远端请求，以单独验证本地应用事务。
    app.documents[0]
        .state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .execution
        .notify_downstream = false;
    let cases: &[&str] = if real {
        &["hardware", "warp"]
    } else {
        &[
            "current",
            "output",
            "document",
            "configuration",
            "topology",
            "cancel",
            "replaced",
        ]
    };
    for reason in cases {
        let bootstrap = crate::graph_requests::Request {
            id: shader_tests::id("bootstrap"),
            generation: 1,
        };
        let mut results = Results::new(unit.clone());
        results.begin(bootstrap.clone());
        results
            .finish(
                &bootstrap,
                crate::desktop::loom_output_batch::Batch::local_image(image.clone()).unwrap(),
            )
            .unwrap();
        let previous = results.output_stamp();
        *app.documents[0].state.execution_results.borrow_mut() = results;
        let mut inputs = PreparedBatch::empty_for_test(
            unit.clone(),
            &app.documents[0].state.canvas.borrow().editor,
        );
        inputs
            .set_dependencies(vec![(
                source.clone(),
                SourceStamp::Output(
                    app.documents[1]
                        .state
                        .execution_results
                        .borrow()
                        .output_stamp(),
                ),
            )])
            .unwrap();
        let request = app
            .executions
            .begin(
                unit.clone(),
                crate::desktop::document_state::new_unit_id().unwrap(),
            )
            .unwrap();
        *app.documents[0].state.execution_configuration.borrow_mut() =
            Some(crate::desktop::execution_configuration::Binding {
                request: request.clone(),
                stamp: crate::desktop::configuration_stamp::Stamp::capture(&app.documents[0].state),
                topology: app.workspace.topology_stamp(),
            });
        app.documents[0]
            .state
            .execution_results
            .borrow_mut()
            .begin(request.clone());
        let release = Arc::new(AtomicBool::new(false));
        if real {
            let backend = if *reason == "hardware" {
                crate::shader_angle::Backend::D3d11
            } else {
                crate::shader_angle::Backend::Warp
            };
            app.execution_jobs
                .start_shader(
                    crate::desktop::shader_execution_job::Execution {
                        unit: unit.clone(),
                        request: request.clone(),
                        inputs,
                        shader: shader_tests::runtime::invocation(backend).into(),
                    },
                    &app.executions,
                )
                .unwrap();
        } else {
            let released = release.clone();
            let (started, receiver) = std::sync::mpsc::channel();
            app.execution_jobs
                .start_shader_test(
                    unit.clone(),
                    request.clone(),
                    inputs,
                    &app.executions,
                    move |cancel| {
                        started.send(()).unwrap();
                        let deadline = Instant::now() + Duration::from_secs(10);
                        while !released.load(Ordering::Acquire) && !cancel.load(Ordering::Acquire) {
                            assert!(
                                Instant::now() < deadline,
                                "local application runner not released"
                            );
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        shader_tests::output(cancel)
                    },
                )
                .unwrap();
            receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        let mut old_source = None;
        let mut old_editor = None;
        let mut replacement = None;
        match *reason {
            "output" => {
                old_source = Some(std::mem::replace(
                    &mut *app.documents[1].state.execution_results.borrow_mut(),
                    Results::new(source.clone()),
                ))
            }
            "document" => {
                old_editor = Some(std::mem::take(
                    &mut app.documents[0].state.canvas.borrow_mut().editor,
                ))
            }
            "configuration" => app.documents[0].state.accept_upstream.set(!original_accept),
            "topology" => app
                .workspace
                .add_units([shader_tests::id("temporary-shader-topology")])
                .unwrap(),
            "cancel" => unsafe {
                crate::desktop::execution_cancel::request(
                    app.documents[0].window.0,
                    &app.documents[0].state,
                )
                .unwrap()
            },
            "replaced" => {
                let next = app
                    .executions
                    .begin(unit.clone(), shader_tests::id("replacement-shader"))
                    .unwrap();
                app.documents[0]
                    .state
                    .execution_results
                    .borrow_mut()
                    .begin(next.clone());
                // 替换请求有自己的绑定，旧报告不能清空它。
                app.documents[0]
                    .state
                    .execution_configuration
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .request = next.clone();
                replacement = Some(next);
            }
            _ => {}
        }
        release.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_secs(25);
        while !app.execution_jobs.is_empty() {
            unsafe {
                app.pending_windows(false).unwrap();
            }
            assert!(
                Instant::now() < deadline,
                "local application completion timeout: {reason}"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        let results = app.documents[0].state.execution_results.borrow();
        if matches!(*reason, "current" | "hardware" | "warp") {
            assert_eq!(results.status(), Status::Completed);
            assert!(!previous.matches(&results));
            let output = results.image_source("output_image").unwrap();
            assert_eq!(
                oracle::hash(&oracle::decode_rgba(
                    output.image.encoded_png().unwrap(),
                    256,
                    256
                )),
                oracle::STRAIGHT_SHA256
            );
            let expected =
                shader_image::prepare(oracle::matrix(), &AtomicBool::new(false)).unwrap();
            assert_eq!(output.image.pixels, expected.image.pixels);
            for index in [0, 2] {
                assert!(app.documents[index]
                    .state
                    .resolved_display
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .image
                    .pixels
                    .same_storage(&output.image.pixels));
            }
        } else {
            assert!(
                previous.matches(&results),
                "stale local output published: {reason}"
            );
            match *reason {
                "replaced" => {
                    assert_eq!(results.active(), replacement.as_ref());
                    assert_eq!(app.executions.active(&unit), replacement.as_ref());
                    assert!(app.documents[0]
                        .state
                        .execution_configuration
                        .borrow()
                        .as_ref()
                        .is_some_and(|binding| Some(&binding.request) == replacement.as_ref()));
                }
                "configuration" | "topology" | "cancel" => {
                    assert_eq!(results.status(), Status::Cancelled)
                }
                _ => assert_eq!(results.status(), Status::Failed),
            }
        }
        drop(results);
        assert!(app.input_jobs.is_empty());
        assert!(app
            .documents
            .iter()
            .all(|doc| doc.state.pending_start.borrow().is_none()
                && doc.state.pending_preparation.borrow().is_none()
                && doc.state.pending_execution.borrow().is_none()));
        if let Some(previous) = old_source {
            *app.documents[1].state.execution_results.borrow_mut() = previous;
        }
        if let Some(previous) = old_editor {
            app.documents[0].state.canvas.borrow_mut().editor = previous;
        }
        if let Some(next) = replacement {
            app.executions.finish(&unit, &next);
        }
        app.workspace = original_workspace.clone();
        app.documents[0].state.accept_upstream.set(original_accept);
        app.documents[0]
            .state
            .execution_configuration
            .borrow_mut()
            .take();
        eprintln!("native-local-shader-application: case={reason} main-loop=true stamps=true no-new-network=true real-child={real}");
    }
    *app.documents[0].state.execution_results.borrow_mut() = original;
    *app.documents[0].state.execution_configuration.borrow_mut() = binding;
    app.documents[0]
        .state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .execution = execution;
    app.sync_display_frames();
}
