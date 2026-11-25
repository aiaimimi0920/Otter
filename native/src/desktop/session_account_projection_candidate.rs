//! v2 已接受图片复用后台单钉图准备；不借用另一条接收路径的回执。
use super::*;
use crate::projection_account::{scene::Binding, Content};
pub(in crate::desktop) fn prepare(
    mut content: Content,
    binding: &Binding,
    space: crate::session_placement::Space,
    rect: Rect,
    target: crate::session_append::Target<'_>,
    cancel: &AtomicBool,
) -> Result<(PreparedRestore, crate::pixels::Pixels)> {
    check(cancel)?;
    if !binding.matches_content(&content) || !rect.valid() {
        return Err(Error::new(
            E_INVALIDARG,
            "Account projection binding changed",
        ));
    }
    let value = content
        .image
        .take()
        .ok_or_else(|| Error::new(E_INVALIDARG, "Missing account projection PNG"))?;
    let png = value
        .png(cancel)
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid account projection PNG"))?;
    let image = crate::image_decode::Image::from_projection_png(
        &png,
        value.width(),
        value.height(),
        &|| cancel.load(Ordering::Acquire),
    )?;
    let encoded = crate::loom_base64::encode_png_data_url(&png, cancel)
        .map_err(|_| Error::new(E_INVALIDARG, "Cannot encode account projection source"))?;
    let serde_json::Value::Object(mut fields) = serde_json::json!({
        "id":binding.unit(),"type":"sticker","x":rect.x,"y":rect.y,"w":rect.w,"h":rect.h,
        "params":{},"opacityNormal":1.0,"opacityMini":0.9
    }) else {
        unreachable!()
    };
    fields.insert(
        crate::projection_account::scene::FIELD.into(),
        serde_json::to_value(binding)
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid account projection record"))?,
    );
    let mut window = crate::session_write::WindowCandidate::new(fields);
    window.insert_encoded("src", encoded);
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Session {
        document_schema_version: u32,
        document_revision: u64,
        otter_native_coordinate_space: serde_json::Value,
        otter_native_projection_scope: crate::session_projection_scope::Scope,
        stickers: [crate::session_write::WindowCandidate; 1],
        links: [(); 0],
        groups: [(); 0],
    }
    let mut coordinates = serde_json::Map::new();
    crate::session_coordinates::install(&mut coordinates, space)
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid account projection coordinates"))?;
    let bytes = crate::session_write::encode_bounded(
        &Session {
            document_schema_version: 1,
            document_revision: 0,
            otter_native_coordinate_space: coordinates
                .remove(crate::session_coordinates::KEY)
                .unwrap(),
            otter_native_projection_scope: binding.scope(),
            stickers: [window],
            links: [],
            groups: [],
        },
        cancel,
    )
    .map_err(|_| Error::new(E_INVALIDARG, "Cannot prepare account projection session"))?;
    let unit = UnitId::parse(binding.unit())
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid account projection Unit"))?;
    PreparedRestore::projected_sticker(bytes, unit, image, cancel, |document| {
        target
            .append(document, cancel)
            .map_err(|error| Error::new(windows::Win32::Foundation::E_FAIL, error))
    })
}
