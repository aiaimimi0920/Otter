//! Real buttons for ordinary image input; retain the original single-file queue and configuration transaction without changing the Canvas source image.
use super::*;
pub(super) unsafe fn build(b: &mut Builder<'_>, model: &model::Model) -> Result<()> {
    let Some(config) = &model.sticker else {
        return Ok(());
    };
    use desktop::sticker_input::Action;
    b.localized_label(match &config.asset {
        Some(asset) => language::Caption::message(
            "parameter-image-file",
            &[("filename", asset.filename.clone())],
        ),
        None => language::Caption::plain("parameter-image-empty"),
    })?;
    b.y += 26;
    for (key, label, action, enabled) in [
        (
            "sticker:upload",
            "parameter-upload-image",
            Action::Upload,
            true,
        ),
        (
            "sticker:clear",
            "parameter-clear-image",
            Action::Clear,
            config.asset.is_some(),
        ),
        (
            "sticker:disable",
            if config.disabled {
                "parameter-enable-image"
            } else {
                "parameter-disable-image"
            },
            Action::ToggleDisabled,
            true,
        ),
    ] {
        b.localized_button(
            key,
            language::Caption::plain(label),
            12,
            416,
            Kind::Sticker(model.stamp.clone(), action),
            enabled,
        )?;
        b.y += 32;
    }
    Ok(())
}
