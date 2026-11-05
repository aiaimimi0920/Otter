//! 本地任务的身份/容量/取消/异常事务；runner 替身不作为真实 GLSL 平台证据。
use super::*;
use crate::{
    desktop::{
        loom_input_job::PreparedBatch,
        loom_output_batch::Batch,
        loom_result_state::{Results, Status},
        shader_execution_job as shader,
        shader_image::{self, tests as oracle},
    },
    editor::Editor,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
pub(crate) static SERIAL: Mutex<()> = Mutex::new(());

impl Tasks {
    pub(crate) fn start_shader_test(
        &mut self,
        unit: UnitId,
        request: Request,
        inputs: PreparedBatch,
        registry: &Requests,
        run: impl FnOnce(&AtomicBool) -> Result<Batch, shader::Error> + Send + 'static,
    ) -> Result<(), Error> {
        let freshness = inputs.freshness().map_err(|_| Error::Allocation)?;
        self.start_work(
            unit.clone(),
            request.clone(),
            freshness,
            registry,
            |notify| {
                let mut job = shader::Job::default();
                job.start_runner(unit, request, inputs, notify, run)
                    .map_err(Error::ShaderStart)?;
                Ok(Work::Shader(job))
            },
        )
    }
}
pub(crate) fn id(text: &str) -> UnitId {
    UnitId::parse(text).unwrap()
}
pub(crate) fn output(cancel: &AtomicBool) -> Result<Batch, shader::Error> {
    shader_image::prepare(oracle::matrix(), cancel)
        .map_err(shader::Error::Image)?
        .into_batch()
        .map_err(shader::Error::Output)
}
pub(crate) fn wait(tasks: &mut Tasks, registry: &Requests) -> Report {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        tasks.poll();
        if let Some(report) = tasks.take_settled(registry) {
            return report;
        }
        assert!(Instant::now() < deadline, "local Shader task timeout");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn start(
    tasks: &mut Tasks,
    registry: &mut Requests,
    editor: &Editor,
    run: impl FnOnce(&AtomicBool) -> Result<Batch, shader::Error> + Send + 'static,
) -> Request {
    let request = registry.begin(id("local"), id("local-request")).unwrap();
    tasks
        .start_shader_test(
            id("local"),
            request.clone(),
            PreparedBatch::empty_for_test(id("local"), editor),
            registry,
            run,
        )
        .unwrap();
    request
}

#[test]
fn local_shader_completion_commits_once_and_rejects_forged_or_stale_identity() {
    let _serial = SERIAL.lock().unwrap();
    let editor = Editor::default();
    let mut tasks = Tasks::default();
    let mut registry = Requests::default();
    let request = start(&mut tasks, &mut registry, &editor, output);
    let mut results = Results::new(id("local"));
    results.begin(request.clone());
    let mut report = wait(&mut tasks, &registry);
    let replacement = Editor::default();
    assert_eq!(
        report.commit(&mut registry, &mut results, |_| Some(&replacement)),
        Err(CommitError::Stale)
    );
    assert!(results.ports().is_empty());
    let other = registry.begin(id("other"), id("other-request")).unwrap();
    let mut wrong = Results::new(id("other"));
    wrong.begin(other.clone());
    report.unit = id("other");
    report.request = other.clone();
    assert_eq!(
        report.commit(&mut registry, &mut wrong, |_| Some(&editor)),
        Err(CommitError::Stale)
    );
    assert!(wrong.ports().is_empty());
    assert!(registry.is_latest(&id("other"), &other));
    report.unit = id("local");
    report.request = request.clone();
    report
        .commit(&mut registry, &mut results, |_| Some(&editor))
        .unwrap();
    assert_eq!(results.status(), Status::Completed);
    let source = results.image_source("output_image").unwrap();
    assert_eq!(
        oracle::hash(&oracle::decode_rgba(
            source.image.encoded_png().unwrap(),
            256,
            256
        )),
        oracle::STRAIGHT_SHA256
    );
    assert_eq!(
        report.commit(&mut registry, &mut results, |_| Some(&editor)),
        Err(CommitError::Stale)
    );
    assert!(registry.active(&id("local")).is_none());
    assert!(tasks.is_empty());
}

#[test]
fn local_shader_replacement_cancellation_and_retirement_keep_old_output() {
    let _serial = SERIAL.lock().unwrap();
    for reason in ["replaced", "cancelled", "closed", "edited", "late-cancel"] {
        let editor = Editor::default();
        let mut tasks = Tasks::default();
        let mut registry = Requests::default();
        let mut results = Results::new(id("local"));
        let old = Request {
            id: id("old"),
            generation: 1,
        };
        results.begin(old.clone());
        results
            .finish(&old, output(&AtomicBool::new(false)).unwrap())
            .unwrap();
        let stamp = results.output_stamp();
        let request = start(&mut tasks, &mut registry, &editor, output);
        results.begin(request.clone());
        let mut report = if reason == "late-cancel" {
            Some(wait(&mut tasks, &registry))
        } else {
            None
        };
        match reason {
            "replaced" => {
                registry.begin(id("local"), id("replacement")).unwrap();
                tasks.retire_stale(&registry);
            }
            "cancelled" | "late-cancel" => {
                registry.finish(&id("local"), &request);
                tasks.retire_stale(&registry);
            }
            "closed" => tasks.retain_units(|_| false),
            _ => tasks.retire_changed(|_, _| false),
        }
        let mut report = report.take().unwrap_or_else(|| wait(&mut tasks, &registry));
        assert_eq!(
            report.commit(&mut registry, &mut results, |_| Some(&editor)),
            Err(CommitError::Stale)
        );
        assert!(stamp.matches(&results));
        assert!(tasks.is_empty());
    }
}

#[test]
fn local_shader_worker_failures_are_atomic_and_shutdown_reclaims_the_permit() {
    let _serial = SERIAL.lock().unwrap();
    for panic in [false, true] {
        let editor = Editor::default();
        let mut tasks = Tasks::default();
        let mut registry = Requests::default();
        let request = start(&mut tasks, &mut registry, &editor, move |_| {
            assert!(!panic, "injected local Shader worker panic");
            Err(shader::Error::Cancelled)
        });
        let mut results = Results::new(id("local"));
        results.begin(request);
        let stamp = results.output_stamp();
        let mut report = wait(&mut tasks, &registry);
        assert_eq!(report.completion.cleanup_unconfirmed(), panic);
        assert_eq!(
            report.commit(&mut registry, &mut results, |_| Some(&editor)),
            Err(CommitError::ExecutionFailed)
        );
        assert!(report.reject(&mut registry, &mut results, "local execution failed"));
        assert!(stamp.matches(&results));
        assert_eq!(results.status(), Status::Failed);
    }
    let editor = Editor::default();
    let raw: crate::pixels::Pixels = vec![17; 16].into();
    let weak = raw.downgrade();
    let (notify, received) = std::sync::mpsc::channel();
    let mut tasks = Tasks::with_notify(Arc::new(move || {
        let _ = notify.send(weak.upgrade().is_none());
    }));
    let mut registry = Requests::default();
    start(&mut tasks, &mut registry, &editor, move |cancel| {
        let _raw = raw;
        let deadline = Instant::now() + Duration::from_secs(5);
        while !cancel.load(Ordering::Acquire) {
            assert!(
                Instant::now() < deadline,
                "shutdown did not cancel local Shader"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        Err(shader::Error::Cancelled)
    });
    let reports = tasks.shutdown();
    assert!(
        received.recv_timeout(Duration::from_secs(5)).unwrap(),
        "cancel notification precedes captured texture release"
    );
    assert_eq!(reports.len(), 1);
    assert!(!reports[0].current);
    assert!(!reports[0].completion.cleanup_unconfirmed());
    assert!(tasks.is_empty());
    start(&mut tasks, &mut registry, &editor, output);
    assert!(wait(&mut tasks, &registry).current);
}

#[test]
fn local_shader_slots_include_retired_workers_and_unconsumed_results() {
    let _serial = SERIAL.lock().unwrap();
    let editor = Editor::default();
    let mut tasks = Tasks::default();
    let mut registry = Requests::default();
    let release = Arc::new(AtomicBool::new(false));
    let worker_release = release.clone();
    let first = start(&mut tasks, &mut registry, &editor, move |cancel| {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !worker_release.load(Ordering::Acquire) && !cancel.load(Ordering::Acquire) {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        output(cancel)
    });
    assert!(matches!(
        tasks.start_shader_test(
            id("local"),
            first,
            PreparedBatch::empty_for_test(id("local"), &editor),
            &registry,
            output
        ),
        Err(Error::Duplicate)
    ));
    let second = registry.begin(id("second"), id("request2")).unwrap();
    let second_release = release.clone();
    tasks
        .start_shader_test(
            id("second"),
            second.clone(),
            PreparedBatch::empty_for_test(id("second"), &editor),
            &registry,
            move |cancel| {
                let deadline = Instant::now() + Duration::from_secs(5);
                while !second_release.load(Ordering::Acquire) && !cancel.load(Ordering::Acquire) {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(1));
                }
                output(cancel)
            },
        )
        .unwrap();
    let mut other = Tasks::default();
    let third = registry.begin(id("third"), id("request3")).unwrap();
    assert!(matches!(
        other.start_shader_test(
            id("third"),
            third.clone(),
            PreparedBatch::empty_for_test(id("third"), &editor),
            &registry,
            output
        ),
        Err(Error::ShaderStart(shader::Error::WorkerLimit))
    ));
    release.store(true, Ordering::Release);
    let deadline = Instant::now() + Duration::from_secs(10);
    while tasks.entries[0].completion.is_none() {
        tasks.poll();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(matches!(
        tasks.start_shader_test(
            id("third"),
            third,
            PreparedBatch::empty_for_test(id("third"), &editor),
            &registry,
            output
        ),
        Err(Error::Capacity)
    ));
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks.shutdown().len(), 2);
    assert!(tasks.is_empty());
}

#[path = "shader_execution_runtime_tests.rs"]
pub(crate) mod runtime;

#[test]
fn local_shader_admission_rejects_before_worker_or_library_loading() {
    use crate::shader_process::{OwnedInvocation, OwnedTexture, TrustedBundle};
    let _serial = SERIAL.lock().unwrap();
    let editor = Editor::default();
    let mut tasks = Tasks::default();
    let mut registry = Requests::default();
    for invalid in ["size", "uniform", "source"] {
        let shader = OwnedInvocation {
            bundle: Arc::new(
                TrustedBundle::new(
                    &std::env::current_dir().unwrap(),
                    crate::shader_angle::tests::fixture_identity(),
                )
                .unwrap(),
            ),
            backend: crate::shader_angle::Backend::D3d11,
            width: if invalid == "size" { 0 } else { 1 },
            height: 1,
            vertex: if invalid == "source" {
                String::new()
            } else {
                crate::shader_angle::tests::VERTEX.into()
            },
            fragment: crate::shader_angle::tests::FRAGMENT.into(),
            textures: vec![OwnedTexture {
                name: "input".into(),
                width: 1,
                height: 1,
                rgba: vec![0; 4].into(),
            }],
            uniforms: vec![(
                "gain".into(),
                if invalid == "uniform" { f32::NAN } else { 1.0 },
            )],
        };
        let raw = shader.textures[0].rgba.downgrade();
        let request = registry.begin(id("local"), id("invalid")).unwrap();
        assert!(matches!(
            tasks.start_shader(
                shader::Execution {
                    unit: id("local"),
                    request: request.clone(),
                    inputs: PreparedBatch::empty_for_test(id("local"), &editor),
                    shader: shader.into()
                },
                &registry
            ),
            Err(Error::ShaderStart(shader::Error::Admission(_)))
        ));
        assert!(raw.upgrade().is_none());
        assert!(tasks.is_empty());
        assert!(
            registry.is_latest(&id("local"), &request),
            "caller still owns failed-start cleanup"
        );
    }
    start(&mut tasks, &mut registry, &editor, output);
    assert!(wait(&mut tasks, &registry).current);
}
