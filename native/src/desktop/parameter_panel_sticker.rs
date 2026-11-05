//! 普通image输入的真实按钮；沿原单文件队列和配置事务，不改Canvas原图。
use super::*;
pub(super) unsafe fn build(b: &mut Builder<'_>, model: &model::Model) -> Result<()> {
    let Some(config) = &model.sticker else {
        return Ok(());
    };
    use desktop::sticker_input::Action;
    b.label(&format!(
        "图片 · {}",
        config
            .asset
            .as_ref()
            .map_or("未选择", |asset| asset.filename.as_str())
    ))?;
    b.y += 26;
    for (key, label, action, enabled) in [
        ("sticker:upload", "上传图片…", Action::Upload, true),
        (
            "sticker:clear",
            "清空图片",
            Action::Clear,
            config.asset.is_some(),
        ),
        (
            "sticker:disable",
            if config.disabled {
                "启用图片输入"
            } else {
                "禁用图片输入"
            },
            Action::ToggleDisabled,
            true,
        ),
    ] {
        b.button(
            key,
            label,
            12,
            416,
            Kind::Sticker(model.stamp.clone(), action),
            enabled,
        )?;
        b.y += 32;
    }
    Ok(())
}
