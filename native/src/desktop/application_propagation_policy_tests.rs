//! 在真实 Application 通知入口核普通贴图策略；不调度网络或执行 worker。
use crate::desktop::{self, capability_selection::Selection};
use std::rc::Rc;

#[test]
fn sticker_notify_controls_both_root_and_relay_without_rebuilding_topology() {
    unsafe {
        let mut app = crate::desktop::application::chrome::tests::app();
        app.add_window(false).unwrap();
        app.add_window(false).unwrap();
        let (_, identity, _) = crate::loom_bridge_client::tests::fixture();
        let handshake = crate::loom_handshake::response(
            &serde_json::to_vec(&crate::loom_handshake::tests::fixture()).unwrap(),
        )
        .unwrap()
        .unwrap();
        *app.documents[2].state.selected_art.borrow_mut() = Some(Selection {
            identity: desktop::document_state::new_unit_id().unwrap(),
            execution: Default::default(),
            parameters: crate::loom_parameter_state::Parameters::new(
                &handshake.capability(0).unwrap(),
            )
            .unwrap(),
            catalog: Rc::new(desktop::capability_job::Loaded {
                identity,
                handshake,
            }),
        });
        app.documents[0].state.canvas.borrow_mut().image = Some(desktop::image::Image {
            width: 1,
            height: 1,
            pixels: vec![0, 0, 0, 255].into(),
        });
        let ids = app
            .documents
            .iter()
            .map(|d| d.state.unit_id.clone())
            .collect::<Vec<_>>();
        for (i, input) in [(0, "image"), (1, "amount")] {
            app.workspace
                .connect(crate::workspace::Link {
                    id: desktop::document_state::new_unit_id().unwrap(),
                    from_unit: ids[i].clone(),
                    from_port: "output_image".into(),
                    to_unit: ids[i + 1].clone(),
                    to_port: input.into(),
                })
                .unwrap();
        }
        let topology = app.workspace.topology_stamp();
        assert_eq!(
            app.notification_targets(&ids[0]).unwrap(),
            vec![ids[2].clone()]
        );
        for index in [0, 1] {
            let state = &app.documents[index].state;
            desktop::node_settings::apply(
                app.documents[index].window.0,
                state,
                desktop::node_settings::Action::Notify,
            )
            .unwrap();
            assert!(app.notification_targets(&ids[0]).unwrap().is_empty());
            app.propagate_created_child(&ids[0]);
            assert!(app.documents[2].state.pending_start.borrow().is_none());
            assert!(state.accept_upstream.get() && !state.sticker_input.snapshot().disabled);
            desktop::node_settings::apply(
                app.documents[index].window.0,
                state,
                desktop::node_settings::Action::Notify,
            )
            .unwrap();
            assert_eq!(
                app.notification_targets(&ids[0]).unwrap(),
                vec![ids[2].clone()]
            );
        }
        app.propagate_created_child(&ids[0]);
        assert!(app.documents[2].state.pending_start.borrow().is_some());
        assert!(topology.matches(&app.workspace));
        assert!(app.execution_jobs.is_empty() && app.input_jobs.is_empty());
        app.shutdown();
    }
}
