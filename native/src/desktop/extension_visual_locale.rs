//! Translate only internally generated availability scenes; external scene content and interaction owners remain unchanged.
use super::*;
use crate::{extension_visual::availability::Unavailable, ui_text::DisplayLanguage};
use std::borrow::Cow;

fn scene(visual: &Visual, language: DisplayLanguage) -> Cow<'_, serde_json::Value> {
    let Some(unavailable) = &visual.unavailable else {
        return Cow::Borrowed(&visual.scene);
    };
    let mut scene = visual.scene.clone();
    match unavailable {
        Unavailable::Scene => {
            scene["props"]["text"] =
                crate::ui_text::text_for(language, "integration-extension-view-unavailable").into();
        }
        Unavailable::Attachments(labels) => {
            let children = scene["children"]
                .as_array_mut()
                .expect("internal unavailable attachment scene has children");
            assert_eq!(children.len(), labels.len());
            for (child, label) in children.iter_mut().zip(labels) {
                child["props"]["text"] = crate::ui_text::format_for(
                    language,
                    "integration-extension-data-unavailable",
                    &[("type", label.as_str().into())],
                )
                .into();
            }
        }
    }
    Cow::Owned(scene)
}

pub(super) fn prepare(
    visual: &Visual,
    unit_size: [f32; 2],
    language: DisplayLanguage,
) -> Result<Presentation> {
    let bytes = serde_json::to_vec(scene(visual, language).as_ref())
        .map_err(|_| windows::core::Error::new(E_INVALIDARG, "Invalid extension scene"))?;
    let scene = crate::surface_scene::Scene::parse(&bytes)
        .map_err(|_| windows::core::Error::new(E_INVALIDARG, "Invalid extension scene"))?;
    let bounds = visual.bounds.resolve(unit_size);
    let size = [bounds[2].max(1.), bounds[3].max(1.)];
    let mut presentation = Presentation::new(
        scene,
        crate::surface_viewport::style(),
        crate::surface_viewport::context(size),
        size,
    )?;
    presentation.embed_controls()?;
    Ok(presentation)
}

impl Views {
    pub(in crate::desktop) fn refresh_language(&self, language: DisplayLanguage) -> Result<bool> {
        if !self.valid.get() || self.language.get() == language {
            return Ok(false);
        }
        // Prepare every internal replacement first. Failed preparation cannot publish a mixed language.
        let replacements = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, (visual, _))| visual.unavailable.is_some())
            .map(|(index, (visual, _))| {
                prepare(visual, self.size.get(), language).map(|presentation| (index, presentation))
            })
            .collect::<Result<Vec<_>>>()?;
        let changed = !replacements.is_empty();
        for (index, presentation) in replacements {
            *self.items[index].1.borrow_mut() = presentation;
        }
        // Internal fallbacks have no selectable roots or click routes; external hit regions and presses survive.
        self.language.set(language);
        Ok(changed)
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "extension_overlay_locale_tests.rs"]
mod overlay_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "extension_visual_locale_tests.rs"]
mod tests;
