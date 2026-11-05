//! 两个真实缓存命中共用任务两槽和单 child 容量；取消等待者不能连接 Loom 或丢弃另一项。
use super::*;
use crate::desktop::{
    loom_execution_tasks::{shader_tests, CommitError, Completion, Tasks},
    loom_input_job::Source,
    loom_result_state::{Results, Status},
};
#[test]
#[ignore = "requires verified ANGLE fixture; two cached local workers, bounded real children and hardware/WARP"]
fn shader_recipe_two_cached_workers_wait_for_single_child_and_cancel_without_network() {
    let _serial = shader_tests::SERIAL.lock().unwrap();
    let _input = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    for backend in [Backend::D3d11, Backend::Warp] {
        for cancel_first in [false, true] {
            let editor = Editor::default();
            let (listener, identity, _) = crate::loom_bridge_client::tests::fixture();
            listener.set_nonblocking(true).unwrap();
            let mut cache = Cache::default();
            let mut tasks = Tasks::default();
            let mut registry = crate::graph_requests::Requests::default();
            let mut results = Vec::new();
            let capacity = crate::shader_process::CAPACITY
                .reserve(crate::shader_process::CHILD_LIMIT)
                .expect("exclusive child capacity before test");
            for name in ["cached-a", "cached-b"] {
                let mut state = fixture_state();
                state.unit_id = staging::id(name);
                cache.insert(
                    Key::capture(&state, Context::Independent).unwrap(),
                    payload(),
                );
                let mut job = execution(&state);
                job.identity = identity.clone();
                job.shader = Some(staging::runtime(backend, true));
                job.inputs = staging::prepare_sources(
                    &editor,
                    vec![Source::capture(
                        "input_path".into(),
                        staging::id("source"),
                        1,
                        1,
                        vec![11, 23, 37, 255].into(),
                        &editor,
                    )
                    .unwrap()],
                );
                job.inputs.target = state.unit_id.clone();
                job.request = registry
                    .begin(state.unit_id.clone(), staging::id(name))
                    .unwrap();
                let mut output = Results::new(state.unit_id.clone());
                output.begin(job.request.clone());
                results.push((state.unit_id.clone(), output));
                cache.bind(&state, &mut job);
                assert!(job.shader.as_ref().unwrap().is_cached());
                tasks.start(job, &registry).unwrap();
            }
            assert_eq!(
                tasks.len(),
                2,
                "second cache hit must not be consumed as WorkerLimit"
            );
            tasks.poll();
            assert!(tasks.take_settled(&registry).is_none());
            if cancel_first {
                let unit = staging::id("cached-a");
                let request = registry.active(&unit).unwrap().clone();
                assert!(registry.finish(&unit, &request));
                tasks.retire_stale(&registry);
                // 容量仍被占满时必须回收取消者，不能仅凭最终 Stale 掩盖一次多余渲染。
                let mut report = shader_tests::wait(&mut tasks, &registry);
                assert_eq!(report.unit, unit);
                assert!(matches!(
                    &report.completion,
                    Completion::Shader(Err(crate::desktop::shader_execution_job::Error::Cancelled))
                ));
                assert_eq!(
                    report.commit(&mut registry, &mut results[0].1, |_| Some(&editor)),
                    Err(CommitError::Stale)
                );
                assert!(results[0].1.ports().is_empty());
                assert_eq!(tasks.len(), 1);
                assert_eq!(
                    crate::shader_process::CAPACITY.used(),
                    crate::shader_process::CHILD_LIMIT
                );
            }
            drop(capacity);
            let mut committed = 0;
            while !tasks.is_empty() {
                let mut report = shader_tests::wait(&mut tasks, &registry);
                assert!(matches!(&report.completion, Completion::Shader(_)));
                let (_, output) = results
                    .iter_mut()
                    .find(|(unit, _)| unit == &report.unit)
                    .unwrap();
                report
                    .commit(&mut registry, output, |_| Some(&editor))
                    .unwrap();
                assert_eq!(output.status(), Status::Completed);
                let image = output.image_source("output").unwrap().image;
                assert_eq!(&*image.pixels, &[11, 23, 37, 255]);
                assert_eq!(
                    &*staging::decode_png(&staging::encoded(image.encoded_png().unwrap())).rgba,
                    &[37, 23, 11, 255]
                );
                committed += 1;
            }
            assert_eq!(committed, if cancel_first { 1 } else { 2 });
            assert!(registry.is_idle());
            assert_eq!(crate::shader_process::CAPACITY.used(), 0);
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            println!("native-shader-cache-workers: backend={backend:?} workers=2 child-capacity=1 cancel-first={cancel_first} committed={committed} network=false cleanup=true");
        }
    }
}
