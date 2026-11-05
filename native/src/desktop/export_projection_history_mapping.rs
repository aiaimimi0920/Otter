//! 暴露既有Layout的自然像素→历史可见框映射，不复制传播/裁剪布局算法。
use super::*;

impl Frame {
    pub(in crate::desktop) fn history_mapping(
        &self,
        image: &Image,
        crop: Option<Rect>,
    ) -> Result<Sampling> {
        let layout = Layout::for_frame(image, self, crop)?;
        let projection = match layout.sampling {
            Sampling::Empty => return Ok(Sampling::Empty),
            Sampling::Full => Projection {
                source: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: image.width as f64,
                    height: image.height as f64,
                },
                target: Rect {
                    x: layout.base.left as f64,
                    y: layout.base.top as f64,
                    width: (layout.base.right - layout.base.left) as f64,
                    height: (layout.base.bottom - layout.base.top) as f64,
                },
            },
            Sampling::Region(projection) => projection,
        };
        let target = Rect {
            x: projection.target.x - layout.visible.x,
            y: projection.target.y - layout.visible.y,
            ..projection.target
        };
        Projection::clipped(projection.source, target, (image.width, image.height))
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid history source mapping"))
    }
}
