//! Internal fallbacks translate without rewriting external data or replacing interaction owners.
use super::*;
use crate::extension_visual::{self, availability_tests, tests as fixtures};
use serde_json::json;

struct LanguageReset(DisplayLanguage);
impl LanguageReset {
    fn english() -> Self {
        let reset = Self(crate::ui_text::current_language());
        crate::ui_text::set_language(DisplayLanguage::English);
        reset
    }
}
impl Drop for LanguageReset {
    fn drop(&mut self) {
        crate::ui_text::set_language(self.0);
    }
}
fn unavailable_scene() -> Visual {
    let mut catalog = fixtures::catalog_value();
    catalog["contributions"]["renderers"][0]["payload"]["payload"]["attachmentScenePath"] =
        "/absent".into();
    let catalog =
        crate::loom_extension_snapshot::Snapshot::parse(&serde_json::to_vec(&catalog).unwrap())
            .unwrap();
    extension_visual::project(Some(&catalog), &availability_tests::text_store(), 4).remove(0)
}
fn unavailable_data() -> Visual {
    extension_visual::project(None, &availability_tests::text_store(), 4).remove(0)
}
fn text(views: &Views, index: usize, id: &str) -> String {
    views.items[index]
        .1
        .borrow()
        .scene()
        .root()
        .visible_node(id)
        .unwrap()
        .props()["text"]
        .as_str()
        .unwrap()
        .into()
}
fn external() -> Visual {
    let mut visual = extension_visual::project(
        Some(&fixtures::catalog()),
        &availability_tests::text_store(),
        4,
    )
    .remove(0);
    visual.bounds = Bounds::Fixed([0., 0., 240., 40.]);
    visual.scene = json!({
        "id":"extension-unavailable","type":"text",
        "props":{"text":"First Second 扩展视图不可用","selectable":true},
        "layout":{"width":240,"height":40}
    });
    visual
}

#[test]
fn generated_scene_and_badge_switch_languages_without_changing_business_metadata() {
    let _language = LanguageReset::english();
    let views =
        Views::prepare(vec![unavailable_scene(), unavailable_data()], [400., 200.]).unwrap();
    assert!(!views.items[0].0.placeholder);
    assert!(views.items[0].0.local_copy);
    assert!(views.items[1].0.placeholder);
    let type_id = extension_visual::OCR_RESULT;
    for (language, scene_label, data_label) in [
        (
            DisplayLanguage::English,
            "Extension view unavailable",
            format!("Extension data unavailable · \u{2068}{type_id}\u{2069}"),
        ),
        (
            DisplayLanguage::SimplifiedChinese,
            "扩展视图不可用",
            format!("扩展数据不可用 · \u{2068}{type_id}\u{2069}"),
        ),
        (
            DisplayLanguage::English,
            "Extension view unavailable",
            format!("Extension data unavailable · \u{2068}{type_id}\u{2069}"),
        ),
    ] {
        views.refresh_language(language).unwrap();
        assert_eq!(text(&views, 0, "extension-unavailable"), scene_label);
        assert_eq!(text(&views, 1, "unavailable-0"), data_label);
        assert!(views.rects().is_empty());
        assert!(views.hit([8., 8.]).is_none());
        assert!(!views.refresh_language(language).unwrap());
    }
}

#[test]
fn external_matching_text_and_active_selection_survive_same_owner_refresh() {
    let _language = LanguageReset::english();
    let views = Rc::new(
        Views::prepare(
            vec![external(), unavailable_scene(), unavailable_data()],
            [400., 200.],
        )
        .unwrap(),
    );
    let shared = views.clone();
    let identity = views.identity();
    let scene = views.items[0].0.scene.clone();
    let rects = views.rects().to_vec();
    let start = [rects[0].left + 0.1, rects[0].top + 2.];
    assert!(views.begin(start, false).unwrap());
    views.update([230., start[1]]).unwrap();
    let selected = views.selected_text().unwrap().unwrap();
    assert!(!selected.is_empty());
    assert!(views.dragging());
    assert!(views
        .refresh_language(DisplayLanguage::SimplifiedChinese)
        .unwrap());
    assert!(Rc::ptr_eq(&views, &shared));
    assert_eq!(views.identity(), identity);
    assert!(views.dragging());
    assert_eq!(
        views.selected_text().unwrap().as_deref(),
        Some(selected.as_str())
    );
    assert_eq!(views.items[0].0.scene, scene);
    assert_eq!(
        text(&views, 0, "extension-unavailable"),
        "First Second 扩展视图不可用"
    );
    assert_eq!(&*views.rects(), &rects);
    views.cancel(true);
    let only_external = Views::prepare(vec![external()], [400., 200.]).unwrap();
    assert!(!only_external
        .refresh_language(DisplayLanguage::SimplifiedChinese)
        .unwrap());
    assert_eq!(
        only_external.language.get(),
        DisplayLanguage::SimplifiedChinese
    );
    // Explicit invalidation remains authoritative even when a new language is requested.
    only_external.valid.set(false);
    assert!(!only_external
        .refresh_language(DisplayLanguage::English)
        .unwrap());
}

#[test]
fn failed_preparation_leaves_every_presentation_and_language_unchanged() {
    let _language = LanguageReset::english();
    let mut views =
        Views::prepare(vec![unavailable_scene(), unavailable_scene()], [400., 200.]).unwrap();
    views.items[1].0.scene["type"] = "invalid-node".into();
    assert!(views
        .refresh_language(DisplayLanguage::SimplifiedChinese)
        .is_err());
    assert_eq!(views.language.get(), DisplayLanguage::English);
    for index in 0..2 {
        assert_eq!(
            text(&views, index, "extension-unavailable"),
            "Extension view unavailable"
        );
    }
}
