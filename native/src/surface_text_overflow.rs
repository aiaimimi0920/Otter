//! 仅最终文字帧使用省略；固有测量、完整 UTF-16 和盒几何仍归原布局 owner。
use super::*;
use crate::surface_scene::{Kind, Node};

impl Engine {
    pub(crate) fn layout_final(
        &self,
        node: &Node,
        text: &str,
        style: Style<'_>,
        width: f32,
        height: f32,
    ) -> Result<Layout> {
        let ellipsis = match node.style().get("textOverflow") {
            None => false,
            Some(value) => match value.as_str() {
                Some("clip") => false,
                Some("ellipsis") => true,
                _ => return Err(invalid()),
            },
        };
        if ellipsis
            && (node.kind() != Kind::Text
                || style.whitespace != Whitespace::Nowrap
                || !matches!(style.align, Align::Start)
                || ["overflowX", "overflowY"]
                    .iter()
                    .any(|key| node.layout().get(*key).and_then(|v| v.as_str()) != Some("hidden")))
        {
            // 多行/其他对齐的 CSS inline-end 与 DWrite 行尾 trimming 不等价，不能静默近似。
            return Err(invalid());
        }
        let layout = self.layout(text, style, width, height)?;
        if ellipsis {
            // nowrap 仍可能含 Unicode 强制分隔符，不能把实际多行误当单行开放。
            if layout.line_count() != 1 {
                return Err(invalid());
            }
            // 此 layout 尚未共享；绝不原地修改上一帧或固有测量的 COM 对象。
            // text-overflow 不截断纵向末行：native 高度至少容纳整行，实际像素由 TextItem 裁剪。
            // 保留 trimming 前 metrics，省略只影响绘制/命中，不缩小固有宽度或溢出几何。
            unsafe {
                layout
                    .native
                    .SetMaxHeight(height.max(layout.metrics.height))?;
                let sign = self.factory.CreateEllipsisTrimmingSign(&layout.native)?;
                layout.native.SetTrimming(
                    &DWRITE_TRIMMING {
                        granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                        ..Default::default()
                    },
                    &sign,
                )?;
            }
        }
        Ok(layout)
    }
}
impl Layout {
    /// DWrite 的 trimming sign 可作为零高 inline hit 返回；用同一逻辑范围的文字命中
    /// 恢复真实行高，不能把 isInside=false 直接当成省略号不可选，也不能扩大到整盒。
    pub(super) fn normalize_trimmed_hit(&self, hit: &mut DWRITE_HIT_TEST_METRICS) -> Result<()> {
        if !hit.isTrimmed.as_bool() || (hit.isText.as_bool() && hit.height > 0.) {
            return Ok(());
        }
        if hit.textPosition >= self.text_len() {
            return Err(invalid());
        }
        let (mut x, mut y) = (0., 0.);
        let mut text_hit = DWRITE_HIT_TEST_METRICS::default();
        unsafe {
            self.native.HitTestTextPosition(
                hit.textPosition,
                false,
                &mut x,
                &mut y,
                &mut text_hit,
            )?;
        }
        if !text_hit.isTrimmed.as_bool() || text_hit.height <= 0. || text_hit.length == 0 {
            return Err(invalid());
        }
        *hit = text_hit;
        Ok(())
    }
    /// 省略号代表整个隐藏后缀；词级选择不能只复制其中一个不可见词。
    pub(super) fn omitted_range(&self, position: u32) -> Result<Option<std::ops::Range<u32>>> {
        if position > self.text_len() {
            return Err(invalid());
        }
        if self.text.is_empty() {
            return Ok(None);
        }
        let (mut x, mut y) = (0., 0.);
        let mut hit = DWRITE_HIT_TEST_METRICS::default();
        unsafe {
            self.native.HitTestTextPosition(
                position.min(self.text_len() - 1),
                false,
                &mut x,
                &mut y,
                &mut hit,
            )?;
        }
        if !hit.isTrimmed.as_bool() {
            return Ok(None);
        }
        let end = hit
            .textPosition
            .checked_add(hit.length)
            .filter(|end| *end <= self.text_len())
            .ok_or_else(invalid)?;
        if hit.length == 0 {
            return Err(invalid());
        }
        Ok(Some(hit.textPosition..end))
    }
}
