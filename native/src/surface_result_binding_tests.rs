//! 真实 HWND 挂载的单次结果注册；失败保留资格，普通执行撤销后不得重复注册。
use super::*;
use crate::{desktop::loom_result_state::Results, graph_requests::Request, workspace_id::UnitId};

#[test]
fn mount_binds_once_and_cannot_restore_writer_after_ordinary_execution() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let mut results = Results::new(UnitId::parse(window.binding().node()).unwrap());
    assert!(window.result_binding_pending());
    assert!(window
        .bind_node_results(0, |binding| {
            results.bind_surface(binding).unwrap();
            Ok(true)
        })
        .unwrap());
    assert!(!window.result_binding_pending());
    assert_eq!(results.surface_result_revision(), Some(0));
    let request = Request {
        id: UnitId::parse("request:ordinary").unwrap(),
        generation: 1,
    };
    results.begin(request.clone());
    assert!(results.cancel(&request));
    assert!(!window
        .bind_node_results(0, |_| panic!("spent mount must not register again"))
        .unwrap());
    assert_eq!(results.surface_result_revision(), None);
    assert!(results.bind_surface_if_unbound(window.binding()).is_err());
}

#[test]
fn failed_or_deferred_binding_retains_single_registration() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    assert!(window
        .bind_node_results(0, |_| Err(windows::core::Error::new(
            windows::Win32::Foundation::E_FAIL,
            "node is unavailable"
        )))
        .is_err());
    assert!(window.result_binding_pending());
    assert!(!window.bind_node_results(0, |_| Ok(false)).unwrap());
    assert!(window.result_binding_pending());
    assert!(window.bind_node_results(0, |_| Ok(true)).unwrap());
    assert!(!window.result_binding_pending());
}

#[test]
fn revoked_hidden_or_inflight_window_cannot_call_result_binder() {
    for mode in 0..3 {
        let _com = Apartment::new();
        let mut grant = grant();
        let mut window = active_window(&mut grant);
        let mut owner = None;
        let events = vec![event(&window)];
        if mode == 0 {
            drop(grant);
        }
        if mode == 1 {
            window.close().unwrap();
        }
        if mode == 2 {
            owner = window.begin_batch(&events, 0).unwrap();
        }
        assert!(!window
            .bind_node_results(0, |_| panic!("unauthorized binder invoked"))
            .unwrap());
        assert!(window.result_binding_pending());
        if let Some(owner) = owner {
            assert!(window.finish_batch(&events, owner).is_ok());
            assert!(window.bind_node_results(0, |_| Ok(true)).unwrap());
        }
    }
}
