//! 真实缓存启动在空输入绑定处失败；只验证 worker/租约回收，不代表渲染成功。
use super::*;
use crate::{
    desktop::{
        loom_execution_tasks::{
            reap_fixture::{Gate, WAIT},
            shader_tests::SERIAL,
            Completion, Tasks,
        },
        shader_execution_job as shader, shader_payload,
    },
    graph_requests::Requests,
};

#[test]
fn cached_shader_releases_recipe_before_notice_and_reaps_after_thread_exit() {
    let _serial = SERIAL.lock().unwrap();
    let state = fixture_state();
    let mut cache = Cache::default();
    cache.insert(
        Key::capture(&state, Context::Independent).unwrap(),
        payload(),
    );
    let mut execution = execution(&state);
    cache.bind(&state, &mut execution);
    assert!(execution.shader.as_ref().unwrap().is_cached());
    cache.retain(&[], false);
    assert!(cache.entries.is_empty());
    assert!(
        cache.budget.used() > 0,
        "execution still owns evicted recipe"
    );
    let gate = Gate::new(|| {});
    let mut tasks = Tasks::with_notify(gate.notify.clone());
    let mut registry = Requests::default();
    execution.request = registry
        .begin(state.unit_id.clone(), staging::id("cached-request"))
        .unwrap();
    tasks.start(execution, &registry).unwrap();
    gate.noticed.recv_timeout(WAIT).unwrap();
    assert_eq!(
        cache.budget.used(),
        0,
        "recipe lease outlived completion notice"
    );
    assert!(tasks.deadline().is_some());
    tasks.poll();
    assert!(tasks.take_settled(&registry).is_none());
    assert_eq!(tasks.len(), 1);
    gate.release.send(()).unwrap();
    tasks.test_wait_finished(0);
    tasks.poll();
    let report = tasks.take_settled(&registry).unwrap();
    assert!(report.current);
    assert!(matches!(
        report.completion,
        Completion::Shader(Err(shader::Error::Prepare(shader_payload::Error::Bindings)))
    ));
    assert!(tasks.is_empty());
    assert!(tasks.deadline().is_none());
    assert!(tasks.take_completed(&registry).is_none());
    assert!(gate.noticed.try_recv().is_err());
}
