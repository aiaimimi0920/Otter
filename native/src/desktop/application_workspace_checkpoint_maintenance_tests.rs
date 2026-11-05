//! 自动库维护与保存共用单槽；无变更不发布，真正过期或取消结果仍拒绝。
use super::*;
use serde_json::json;

fn policy(maximum: usize) -> crate::session_library::recycle::policy::Policy {
    crate::settings_source::Source::new(
        "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-maintenance-tests.json".into(),
        serde_json::to_vec(&json!({"cache":{"recycleBinMaxEntries":maximum}})).unwrap(),
    )
    .unwrap()
    .document()
    .recycle_policy()
    .unwrap()
}
unsafe fn start(app: &mut Application, changing: bool) -> crate::session_source::Source {
    if changing {
        let record = app.restored_session.as_mut().unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&record.original).unwrap();
        value["recycleBin"] = json!([
            {"entryId":"old","sourceStickerId":"a","createdAt":"legacy",
                "snapshot":{"id":"a","x":0,"y":0,"w":32,"h":32}},
            {"entryId":"new","sourceStickerId":"a","createdAt":"legacy",
                "snapshot":{"id":"a","x":0,"y":0,"w":32,"h":32}}
        ]);
        record.edit_original(serde_json::to_vec(&value).unwrap().into());
    }
    app.recycle_policy = policy(1);
    let source = app.restored_session.as_ref().unwrap().original.clone();
    app.schedule_recycle_maintenance();
    app.start_recycle_maintenance().unwrap();
    assert!(app.save_job.as_ref().unwrap().1.prune_policy().is_some());
    source
}

#[test]
fn no_op_maintenance_finishes_after_model_change_without_replacing_source_or_dirtying_save() {
    unsafe {
        run(|instance| {
            let mut app = new_app(instance);
            let source = start(&mut app, false);
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
            assert!(!app.save_job.as_ref().unwrap().0.matches(&app));
            app.saved_state = Some(save_state::Stamp::capture(&app).unwrap());
            ready(&app);
            // 走产品真正的领取入口；旧实现会返回E_ABORT并在可见模式打开模态警告。
            app.process_reference(false).unwrap();
            assert!(app.save_job.is_none());
            assert!(!app.recycle_maintenance_pending);
            assert!(source.same_storage(&app.restored_session.as_ref().unwrap().original));
            assert_eq!(app.save_status(), save_state::Status::Unchanged);
            assert_eq!(app.documents[0].state.canvas.borrow().normal_opacity, 0.4);
            assert!(IsWindow(Some(app.documents[0].window.0)).as_bool());
        });
    }
}

#[test]
fn changing_stale_prune_and_cancelled_or_policy_changed_noop_still_reject_without_publication() {
    unsafe {
        run(|instance| {
            for (changing, mode) in [(true, 0), (true, 1), (true, 2), (false, 1), (false, 2)] {
                let mut app = new_app(instance);
                let source = start(&mut app, changing);
                match mode {
                    0 => app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4,
                    1 => app.save_job.as_ref().unwrap().1.cancel(),
                    _ => app.recycle_policy = policy(2),
                }
                ready(&app);
                assert_eq!(app.process_reference(false).unwrap_err().code(), E_ABORT);
                assert!(app.save_job.is_none());
                assert!(!app.recycle_maintenance_pending);
                assert!(source.same_storage(&app.restored_session.as_ref().unwrap().original));
                assert!(IsWindow(Some(app.documents[0].window.0)).as_bool());
                drop(app);
            }
        });
    }
}
