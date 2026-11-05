//! 已恢复候选的源分辨率烘焙入口；不从活动窗口重新获取或猜测原图身份。
use super::*;
#[path = "session_workflow_prepare.rs"]
pub mod workflow;

impl VectorCandidate {
    /// 只接受未继续编辑的恢复候选，防止以保留原图覆盖新擦除；未实现效果显式拒绝。
    pub fn bake_source_png(
        &self,
        unit: &UnitId,
        cancel: &AtomicBool,
    ) -> Result<crate::pixels::Pixels> {
        check(cancel)?;
        let document = session_document::decode(self.candidate.original_json())
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid bake session"))?;
        let window = document
            .windows
            .iter()
            .find(|window| window.id == unit.as_str())
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing bake session node"))?;
        self.bake_window_source_png(window, cancel)
    }

    /// 批量快照使用内容策略已经解析的同一会话节点，避免每张图重解析整份大 JSON。
    /// 仅内部调用；不能把任意外部 Window 与本候选的像素混用。
    fn bake_window_source_png(
        &self,
        window: &crate::session_window::Window<'_>,
        cancel: &AtomicBool,
    ) -> Result<crate::pixels::Pixels> {
        self.bake_window_image_png(window, None, cancel)
    }
    fn bake_window_image_png(
        &self,
        window: &crate::session_window::Window<'_>,
        selected: Option<&crate::desktop::image::Image>,
        cancel: &AtomicBool,
    ) -> Result<crate::pixels::Pixels> {
        check(cancel)?;
        let unit = UnitId::parse(&window.id)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid bake session node"))?;
        let mut border = None;
        let mut radius = 0.0;
        let mut edit_state = serde_json::Value::Null;
        if let Some(raw) = window.image_edit_state {
            let edit: serde_json::Value = serde_json::from_str(raw.get())
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid bake edit state"))?;
            border = super::super::export::source_resolution::frame_border::Border::read(&edit)?;
            radius = super::super::export::source_resolution::frame_clip::radius(&edit)?;
            // 主干合成器不读取 imageEditState.rotation；这是遗留元数据，不应在此
            // 拒绝会话或发明整图旋转。标注自身的 rotation 由标注绘制路径处理。
            edit_state = edit;
        }
        let placement = self
            .candidate
            .placements
            .iter()
            .find(|value| value.unit == unit)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing bake placement"))?;
        if placement.art {
            return Err(Error::new(E_INVALIDARG, "Art result is not a sticker bake"));
        }
        let editor = &self
            .vectors
            .iter()
            .find(|(id, _)| *id == unit)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing bake editor"))?
            .1
            .editor;
        if editor.revision() != 0 || editor.draft().is_some() {
            return Err(Error::new(
                E_ABORT,
                "Bake candidate changed after restoration",
            ));
        }
        let images = self
            .candidate
            .images
            .iter()
            .find(|images| images.unit == unit)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing bake images"))?;
        let source = selected
            .or(images.source.as_ref())
            .or(images.preview.as_ref())
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing bake source"))?;
        let logical = if let Some(crop) = placement.crop {
            crop.editor_crop()
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid bake crop"))?
                .source
        } else {
            let frame = placement
                .expanded_rect()
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid bake frame"))?;
            crate::crop_geometry::Size {
                width: frame.w,
                height: frame.h,
            }
        };
        let geometry = super::super::export::source_resolution::Frame::restored(
            source,
            logical,
            placement.crop,
        )?
        .with_beautify(&edit_state)?;
        // 主干模糊采样使用整个 unit 框，不使用传播内容框；不能随底图一起平移。
        let prepared = super::super::export::source_resolution::source_blur::prepare_restored(
            source,
            editor,
            &geometry,
            window,
            placement.crop,
            cancel,
        )?;
        let mut geometry = geometry.with_content(source, window, placement.crop, &edit_state)?;
        let erased = super::super::export::source_resolution::source_erase::prepare(
            source,
            &mut geometry,
            editor.crop_view().map(|crop| crop.source.rect),
            &edit_state,
            cancel,
        )?;
        super::super::export::source_resolution::png_source_resolution(
            &erased,
            &prepared,
            geometry,
            placement.opacity_normal as f32,
            border,
            radius,
            cancel,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restored_candidate_bakes_natural_size_and_rejects_later_edits() {
        let _com = crate::desktop::Apartment::new().unwrap();
        let cancel = AtomicBool::new(false);
        let mut candidate = prepare(
            br#"{"stickers":[{"id":"s","x":0,"y":0,"w":20,"h":10}],"links":[]}"#.to_vec(),
            None,
            &cancel,
        )
        .unwrap();
        let pixels = [10, 20, 30, 255].repeat(40 * 20);
        candidate.images[0].source = Some(crate::desktop::image::Image {
            width: 40,
            height: 20,
            pixels: pixels.clone().into(),
        });
        let mut candidate = VectorCandidate::prepare(candidate, &cancel).unwrap();
        let unit = UnitId::parse("s").unwrap();
        let png = candidate.bake_source_png(&unit, &cancel).unwrap();
        let decoded = crate::desktop::export::decode_png(png.to_vec());
        assert_eq!((decoded.width, decoded.height), (40, 20));
        assert_eq!(decoded.pixels.as_ref(), pixels);
        candidate.vectors[0]
            .1
            .editor
            .crop_to(crate::crop_geometry::Rect {
                x: 1.0,
                y: 1.0,
                width: 5.0,
                height: 5.0,
            })
            .unwrap();
        assert!(candidate.bake_source_png(&unit, &cancel).is_err());
    }
}
