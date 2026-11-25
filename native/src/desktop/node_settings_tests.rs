//! Current real parameter buttons commit through the same snapshot transaction, protecting complete business semantics from before bottom-settings migration.
use super::*;
use desktop::node_settings::Action;

#[test]
fn actual_upstream_button_combines_both_flags_without_scheduling_execution() {
    unsafe {
        for (upstream, listen) in [(true, true), (true, false), (false, true), (false, false)] {
            let f = Fixture::new();
            {
                let mut selected = f.state.selected_art.borrow_mut();
                let config = &mut selected.as_mut().unwrap().execution;
                config.upstream_driven = upstream;
                config.listen_upstream = listen;
            }
            f.tick();
            f.click("setting:1");
            f.tick();
            let config = f.state.selected_art.borrow().as_ref().unwrap().execution;
            assert_eq!(
                (config.upstream_driven, config.listen_upstream),
                (!(upstream && listen), !(upstream && listen))
            );
            assert!(f.state.pending_start.borrow().is_none());
            assert_eq!(f.panel.ui.model.borrow().settings.len(), 4);
        }
    }
}
#[test]
fn node_endpoints_are_presentation_not_parameter_or_topology_changes() {
    unsafe {
        let f = Fixture::new();
        let config = Stamp::capture(&f.state);
        let document = f.state.canvas.borrow().editor.document_stamp();
        f.click("setting:0");
        assert!(!f.state.node_settings.get().hide_collapsed_endpoints);
        f.tick();
        assert!(f.state.node_settings.get().hide_collapsed_endpoints);
        assert!(config.matches(&f.state));
        assert!(document.matches(&f.state.canvas.borrow().editor));
        assert!(f.state.pending_start.borrow().is_none());
        f.state.group_locked.set(true);
        f.click("setting:0");
        f.tick();
        assert!(f.state.node_settings.get().hide_collapsed_endpoints);
        f.state.group_locked.set(false);
        f.click("setting:0");
        f.tick();
        assert!(!f.state.node_settings.get().hide_collapsed_endpoints);
    }
}
#[test]
fn sticker_notify_is_independent_of_image_relay_and_rejects_stale_intents() {
    unsafe {
        let f = Fixture::new();
        f.state.selected_art.borrow_mut().take();
        f.tick();
        assert_eq!(f.panel.ui.model.borrow().settings.len(), 3);
        let before = Stamp::capture(&f.state);
        f.click("setting:1");
        f.tick();
        assert!(!f.state.node_settings.get().notify_downstream);
        assert!(f.state.accept_upstream.get());
        assert!(!before.matches(&f.state));
        f.panel
            .apply(&f.state, Intent::Setting(before, Action::Notify))
            .unwrap();
        assert!(!f.state.node_settings.get().notify_downstream);
        f.click("setting:2");
        f.tick();
        assert!(!f.state.accept_upstream.get());
        assert!(!f.state.node_settings.get().notify_downstream);
        assert!(f.state.pending_start.borrow().is_none());
    }
}
#[test]
fn art_notify_uses_selection_execution_not_sticker_settings() {
    unsafe {
        let f = Fixture::new();
        f.click("setting:3");
        f.tick();
        assert!(
            !f.state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .execution
                .notify_downstream
        );
        assert!(f.state.node_settings.get().notify_downstream);
        assert!(f.state.pending_start.borrow().is_none());
    }
}
