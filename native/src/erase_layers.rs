//! 内容/仅栅格标注两种擦除事务；矢量标注由 Editor 独立拥有，不在这里展平或重建。
use crate::{
    erase::{self, EraseError},
    pixels::Pixels,
    viewport::{checked_image_bytes, Point},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Content,
    RasterAnnotations,
}

#[derive(Debug, Clone)]
pub struct Layers {
    /// None 保持历史同尺寸合同；Some 描述独立标注纹理，不改变底图逻辑框。
    pub annotation_size: Option<(u32, u32)>,
    pub size: (u32, u32),
    pub base: Pixels,
    pub annotations: Option<Pixels>,
}

impl Layers {
    pub fn annotation_dimensions(&self) -> (u32, u32) {
        self.annotation_size.unwrap_or(self.size)
    }
    pub fn valid(&self) -> bool {
        if checked_image_bytes(self.size.0, self.size.1) != Some(self.base.len()) {
            return false;
        }
        match &self.annotations {
            Some(pixels) => {
                let size = self.annotation_dimensions();
                checked_image_bytes(size.0, size.1) == Some(pixels.len())
            }
            None => self.annotation_size.is_none(),
        }
    }
    /// 全部层都成功后才返回候选快照；任何失败都保留原层及其共享身份。
    pub fn erase(
        &self,
        mode: Mode,
        points: &[Point],
        width: f32,
        cancelled: impl FnMut() -> bool,
    ) -> Result<Self, EraseError> {
        self.erase_in_frame(mode, points, width, None, cancelled)
    }

    /// 底图使用源像素坐标，标注层可独立适配可见框；仍在两层成功后才交付。
    pub fn erase_in_frame(
        &self,
        mode: Mode,
        points: &[Point],
        width: f32,
        annotation_frame: Option<crate::crop_geometry::Rect>,
        cancelled: impl FnMut() -> bool,
    ) -> Result<Self, EraseError> {
        self.erase_in_frames(mode, points, width, None, annotation_frame, cancelled)
    }

    /// 两层逆映射各自逻辑框，擦除笔宽仍为文档单位；取消不会发布半层结果。
    pub fn erase_in_frames(
        &self,
        mode: Mode,
        points: &[Point],
        width: f32,
        base_projection: Option<crate::image_projection::Projection>,
        annotation_frame: Option<crate::crop_geometry::Rect>,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Self, EraseError> {
        if !self.valid() {
            return Err(EraseError::InvalidInput);
        }
        let base = if mode == Mode::Content {
            match base_projection {
                Some(frame) => erase::apply_in_projection(
                    &self.base,
                    self.size,
                    points,
                    width,
                    frame,
                    &mut cancelled,
                )?,
                None => erase::apply(&self.base, self.size, points, width, &mut cancelled)?,
            }
        } else {
            self.base.clone()
        };
        let annotations = self
            .annotations
            .as_ref()
            .map(|image| match annotation_frame {
                Some(frame) => erase::apply_in_frame(
                    image,
                    self.annotation_dimensions(),
                    points,
                    width,
                    frame,
                    &mut cancelled,
                ),
                None => erase::apply_in_frame(
                    image,
                    self.annotation_dimensions(),
                    points,
                    width,
                    crate::crop_geometry::Rect {
                        x: 0.0,
                        y: 0.0,
                        width: self.size.0 as f64,
                        height: self.size.1 as f64,
                    },
                    &mut cancelled,
                ),
            })
            .transpose()?;
        if cancelled() {
            return Err(EraseError::Cancelled);
        }
        Ok(Self {
            annotation_size: self.annotation_size,
            size: self.size,
            base,
            annotations,
        })
    }

    pub fn same_storage(&self, other: &Self) -> bool {
        self.size == other.size
            && self.annotation_dimensions() == other.annotation_dimensions()
            && self.base.same_storage(&other.base)
            && match (&self.annotations, &other.annotations) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_storage(b),
                _ => false,
            }
    }
}

#[cfg(test)]
#[path = "raster_dimensions_tests.rs"]
mod dimension_tests;

#[cfg(test)]
mod tests {
    use super::*;
    fn layers() -> Layers {
        Layers {
            annotation_size: None,
            size: (16, 16),
            base: vec![255; 16 * 16 * 4].into(),
            annotations: Some([64, 64, 64, 128].repeat(16 * 16).into()),
        }
    }
    #[test]
    fn modes_preserve_base_identity_or_erase_both_layers_atomically() {
        let source = layers();
        let points = [Point { x: 8.0, y: 8.0 }];
        let annotations = source
            .erase(Mode::RasterAnnotations, &points, 8.0, || false)
            .unwrap();
        assert!(annotations.base.same_storage(&source.base));
        assert_eq!(
            &annotations.annotations.as_ref().unwrap()[(8 * 16 + 8) * 4..(8 * 16 + 9) * 4],
            &[0; 4]
        );
        let content = source.erase(Mode::Content, &points, 8.0, || false).unwrap();
        assert_eq!(&content.base[(8 * 16 + 8) * 4..(8 * 16 + 9) * 4], &[0; 4]);
        assert_eq!(content.annotations, annotations.annotations);
        assert_eq!(source.base[(8 * 16 + 8) * 4 + 3], 255);
        let mut no_annotations = source.clone();
        no_annotations.annotations = None;
        assert!(no_annotations
            .erase(Mode::RasterAnnotations, &points, 8.0, || false)
            .unwrap()
            .same_storage(&no_annotations));
    }
    #[test]
    fn bad_second_layer_or_cancellation_cannot_publish_a_partial_base() {
        let source = layers();
        let backup = source.clone();
        let mut invalid = source.clone();
        invalid.annotations = Some(vec![0; 4].into());
        assert!(matches!(
            invalid.erase(Mode::Content, &[Point { x: 8.0, y: 8.0 }], 8.0, || false),
            Err(EraseError::InvalidInput)
        ));
        let mut polls = 0;
        assert!(matches!(
            source.erase(Mode::Content, &[Point { x: 8.0, y: 8.0 }], 8.0, || {
                polls += 1;
                polls > 13
            }),
            Err(EraseError::Cancelled)
        ));
        assert!(source.same_storage(&backup));
    }
}
