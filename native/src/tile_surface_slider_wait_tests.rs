//! 串行前缀的逻辑时钟oracle：不sleep、不延长原1500ms准入或120秒跟踪上限。
use super::*;
#[test]
fn slow_prefix_holds_commit_then_fresh_terminal_grants_dispatch_once() {
    for case in 0..4 {
        let f = fixture(scene());
        let mut d = Drag::new(field(&f));
        let now = Instant::now();
        let at = |s| now + Duration::from_secs(s);
        let mut s = f.shared.lock();
        assert!(s
            .actions
            .offer(vec![d.sample(&f.frame, [1, 0]).unwrap().unwrap()], now));
        let input = s.actions.take(&f.state, now).unwrap();
        let id = event(&input)["eventId"].as_str().unwrap();
        assert!(s.actions.offer(vec![d.commit().unwrap().unwrap()], now));
        let waiting = activity(
            &f,
            json!([{"ack":crate::tile_surface_actions::tests::ack_value(id,"running"),"actionId":"adjust","cancelable":true}]),
            Json::Null,
        );
        // HTTP本身超过1500ms时，仍由单在途owner及固定等待上限保留依赖，不发后续。
        assert!(s.actions.take(&waiting, at(2)).is_none());
        assert_eq!(s.actions.pending.len(), 1);
        s.actions
            .attempted(&input, &f.ack(&input, "running"), at(3));
        s.actions
            .observe(waiting.clone(), "origin", "device", at(4));
        assert!(s.actions.take(&waiting, at(4)).is_none());
        assert_eq!(s.actions.pending.len(), 1);
        match case {
            0 => {
                s.actions
                    .observe(f.state.clone(), "origin", "device", at(5));
                let commit = s.actions.take(&f.state, at(6)).unwrap();
                assert_eq!(event(&commit)["event"], "change");
                assert_eq!(event(&commit)["payload"], json!({"value":6.0}));
            }
            1 => {
                s.actions
                    .observe(f.state.clone(), "origin", "device", at(5));
                s.actions
                    .observe(f.state.clone(), "origin", "device", at(6));
                assert!(s.actions.take(&f.state, at(7)).is_none()); // 重复GET不能续1500ms。
                assert!(!d.edit.active());
            }
            2 => {
                s.actions.observe(waiting, "origin", "device", at(124)); // 原ACK跟踪120秒上限。
                assert!(s.actions.pending.is_empty());
                assert!(!d.edit.active());
            }
            _ => {
                let failed = activity(
                    &f,
                    json!([]),
                    json!({"requestId":"request","code":"wall_surface_action_failed"}),
                );
                s.actions.observe(failed, "origin", "device", at(5));
                assert!(s.actions.pending.is_empty());
                assert!(!d.edit.active());
            }
        }
    }
}
