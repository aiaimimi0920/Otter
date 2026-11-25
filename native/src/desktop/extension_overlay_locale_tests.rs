//! Hidden document owners exercise locale refresh without invalidating catalog, source, or input identity.
use crate::{
    desktop::{
        document_window::DocumentWindow,
        extension_document,
        extension_overlay::{refresh_language, sync},
        extension_visuals,
        image::Image,
        State,
    },
    extension_visual::availability_tests,
    ui_text::DisplayLanguage,
};

struct LanguageReset(DisplayLanguage);
impl Drop for LanguageReset {
    fn drop(&mut self) {
        crate::ui_text::set_language(self.0);
    }
}
fn label(views: &extension_visuals::Views) -> String {
    views.items[0]
        .1
        .borrow()
        .scene()
        .root()
        .visible_node("unavailable-0")
        .unwrap()
        .props()["text"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn unchanged_overlay_cache_relabels_in_place_and_real_source_changes_still_revoke() {
    unsafe {
        let _reset = LanguageReset(crate::ui_text::current_language());
        crate::ui_text::set_language(DisplayLanguage::English);
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let instance = crate::desktop::workspace_window_fixture::register();
        let state = State::new(
            Some(Image {
                width: 240,
                height: 140,
                pixels: vec![255; 240 * 140 * 4].into(),
            }),
            true,
        )
        .unwrap();
        *state.extension_document.borrow_mut() =
            extension_document::Owner::restore(availability_tests::text_store(), None);
        let document = DocumentWindow::create(instance, state, false).unwrap();
        let hwnd = document.window.0;
        let state = &document.state;
        assert!(sync(hwnd, state).unwrap());
        let views = state
            .canvas
            .borrow()
            .extension_views
            .as_ref()
            .unwrap()
            .shared();
        let stamp = state.extension_document.borrow().stamp();
        let before = serde_json::to_value(state.extension_document.borrow().store.items()).unwrap();
        assert!(label(&views).starts_with("Extension data unavailable"));
        assert!(!sync(hwnd, state).unwrap());
        crate::ui_text::set_language(DisplayLanguage::SimplifiedChinese);
        assert!(sync(hwnd, state).unwrap());
        assert!(label(&views).starts_with("扩展数据不可用"));
        assert!(std::rc::Rc::ptr_eq(
            &views,
            &state
                .canvas
                .borrow()
                .extension_views
                .as_ref()
                .unwrap()
                .shared()
        ));
        assert!(stamp.matches(&state.extension_document.borrow()));
        assert_eq!(
            serde_json::to_value(state.extension_document.borrow().store.items()).unwrap(),
            before
        );
        assert!(!sync(hwnd, state).unwrap());
        crate::ui_text::set_language(DisplayLanguage::English);
        assert!(refresh_language(hwnd, state).unwrap());
        assert!(label(&views).starts_with("Extension data unavailable"));
        assert!(!refresh_language(hwnd, state).unwrap());
        *state.extension_document.borrow_mut() =
            extension_document::Owner::restore(Default::default(), None);
        crate::ui_text::set_language(DisplayLanguage::SimplifiedChinese);
        assert!(!refresh_language(hwnd, state).unwrap());
        assert!(label(&views).starts_with("Extension data unavailable"));
        assert!(sync(hwnd, state).unwrap());
        assert!(!views.valid.get());
    }
}
