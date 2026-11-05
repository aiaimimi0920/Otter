//! 有界操作历史通过转移 Stroke 所有权撤销/重做，不复制图像或点数组。
use crate::editor::Stroke;
use crate::stroke_style::StrokeStyle;
use std::collections::VecDeque;
#[path = "edit_history_source.rs"]
mod source;

const LIMIT: usize = 50;

pub(crate) struct Document {
    pub elements: Vec<Stroke>,
    pub serial: u32,
    pub raster: Option<crate::erase_layers::Layers>,
    pub crop: Option<crate::editor::CropView>,
    pub logical_size: Option<crate::crop_geometry::Size>,
}

pub(crate) struct Annotations {
    pub elements: Vec<Stroke>,
    pub serial: u32,
}

pub(crate) struct Entry {
    annotations: Option<Box<Annotations>>,
    document: Option<Box<Document>>,
    index: usize,
    occupied: bool,
    stored: Option<Stroke>,
    style: Option<StrokeStyle>,
    rotation: Option<f32>,
    serial: Option<u32>,
    raster: Option<crate::erase_layers::Layers>,
    batch: Option<Vec<(usize, Option<Stroke>)>>,
    replacements: Option<Vec<(usize, Stroke)>>,
    styles: Option<Vec<(usize, StrokeStyle)>>,
    crop: Option<Option<crate::editor::CropView>>,
}

impl Entry {
    pub fn annotations(previous: Annotations) -> Self {
        Self {
            annotations: Some(Box::new(previous)),
            ..Self::added(0)
        }
    }
    pub fn document(previous: Document) -> Self {
        Self {
            document: Some(Box::new(previous)),
            ..Self::added(0)
        }
    }
    pub fn with_crop(mut self, previous: Option<crate::editor::CropView>) -> Self {
        self.crop = Some(previous);
        self
    }
    pub fn cropped(previous: Option<crate::editor::CropView>) -> Self {
        Self {
            crop: Some(previous),
            ..Self::added(0)
        }
    }
    pub fn added(index: usize) -> Self {
        Self {
            index,
            occupied: true,
            stored: None,
            style: None,
            rotation: None,
            serial: None,
            raster: None,
            batch: None,
            replacements: None,
            styles: None,
            crop: None,
            document: None,
            annotations: None,
        }
    }
    pub fn removed(index: usize, stroke: Stroke) -> Self {
        Self {
            index,
            occupied: false,
            stored: Some(stroke),
            style: None,
            rotation: None,
            serial: None,
            raster: None,
            batch: None,
            replacements: None,
            styles: None,
            crop: None,
            document: None,
            annotations: None,
        }
    }
    pub fn styled(index: usize, style: StrokeStyle) -> Self {
        Self {
            index,
            occupied: true,
            stored: None,
            style: Some(style),
            rotation: None,
            serial: None,
            raster: None,
            batch: None,
            replacements: None,
            styles: None,
            crop: None,
            document: None,
            annotations: None,
        }
    }
    pub fn styled_many(styles: Vec<(usize, StrokeStyle)>) -> Self {
        Self {
            styles: Some(styles),
            ..Self::added(0)
        }
    }
    pub fn replaced(index: usize, stroke: Stroke) -> Self {
        Self {
            index,
            occupied: true,
            stored: Some(stroke),
            style: None,
            rotation: None,
            serial: None,
            raster: None,
            batch: None,
            replacements: None,
            styles: None,
            crop: None,
            document: None,
            annotations: None,
        }
    }
    fn retained_points(&self) -> usize {
        self.annotations.as_ref().map_or(0, |annotations| {
            annotations
                .elements
                .iter()
                .map(|s| s.points.len())
                .sum::<usize>()
        }) + self.document.as_ref().map_or(0, |document| {
            document
                .elements
                .iter()
                .map(|stroke| stroke.points.len())
                .sum::<usize>()
        }) + self.stored.as_ref().map_or(0, |s| s.points.len())
            + self.replacements.as_ref().map_or(0, |entries| {
                entries
                    .iter()
                    .map(|(_, stroke)| stroke.points.len())
                    .sum::<usize>()
            })
            + self.batch.as_ref().map_or(0, |entries| {
                entries
                    .iter()
                    .map(|(_, stroke)| stroke.as_ref().map_or(0, |s| s.points.len()))
                    .sum::<usize>()
            })
    }
    pub fn rotated(index: usize, angle: f32) -> Self {
        Self {
            index,
            occupied: true,
            stored: None,
            style: None,
            rotation: Some(angle),
            serial: None,
            raster: None,
            batch: None,
            replacements: None,
            styles: None,
            crop: None,
            document: None,
            annotations: None,
        }
    }
    pub fn serial_added(index: usize, previous: u32) -> Self {
        Self {
            serial: Some(previous),
            ..Self::added(index)
        }
    }
    pub fn raster(previous: crate::erase_layers::Layers) -> Self {
        Self {
            raster: Some(previous),
            ..Self::added(0)
        }
    }
    pub fn rasterized(
        previous: crate::erase_layers::Layers,
        removed: Vec<(usize, Option<Stroke>)>,
    ) -> Self {
        Self {
            batch: Some(removed),
            occupied: false,
            ..Self::raster(previous)
        }
    }
    pub fn removed_many(removed: Vec<(usize, Option<Stroke>)>) -> Self {
        Self {
            batch: Some(removed),
            occupied: false,
            ..Self::added(0)
        }
    }
    pub fn added_many(indices: std::ops::Range<usize>) -> Self {
        Self {
            batch: Some(indices.map(|index| (index, None)).collect()),
            ..Self::added(0)
        }
    }
    pub fn replaced_many(previous: Vec<(usize, Stroke)>) -> Self {
        Self {
            replacements: Some(previous),
            ..Self::added(0)
        }
    }
    pub fn raster_refreshed(
        previous: crate::erase_layers::Layers,
        replacements: Vec<(usize, Stroke)>,
        removed: Vec<(usize, Option<Stroke>)>,
    ) -> Self {
        Self {
            replacements: Some(replacements),
            batch: (!removed.is_empty()).then_some(removed),
            occupied: false,
            ..Self::raster(previous)
        }
    }
    fn swap_replacements(&mut self, elements: &mut [Stroke]) {
        if let Some(entries) = &mut self.replacements {
            for (index, stored) in entries {
                std::mem::swap(stored, &mut elements[*index]);
            }
        }
    }
    fn toggle(
        &mut self,
        elements: &mut Vec<Stroke>,
        serial_count: &mut u32,
        raster: &mut Option<crate::erase_layers::Layers>,
        crop: &mut Option<crate::editor::CropView>,
        logical_size: &mut Option<crate::crop_geometry::Size>,
    ) {
        if let Some(annotations) = &mut self.annotations {
            std::mem::swap(&mut annotations.elements, elements);
            std::mem::swap(&mut annotations.serial, serial_count);
            return;
        }
        if let Some(document) = &mut self.document {
            std::mem::swap(&mut document.elements, elements);
            std::mem::swap(&mut document.serial, serial_count);
            std::mem::swap(&mut document.raster, raster);
            std::mem::swap(&mut document.crop, crop);
            std::mem::swap(&mut document.logical_size, logical_size);
            return;
        }
        if let Some(previous) = &mut self.crop {
            std::mem::swap(previous, crop);
            if self.raster.is_none() {
                return;
            }
        }
        if let Some(styles) = &mut self.styles {
            for (index, style) in styles {
                *style = elements[*index].apply_style(*style);
            }
            return;
        }
        let raster_change = self.raster.is_some();
        if raster_change {
            std::mem::swap(&mut self.raster, raster);
        }
        if self.batch.is_none() || !self.occupied {
            // 替换索引属于移除后的集合：撤销先还原效果，再插回矢量。
            self.swap_replacements(elements);
        }
        if self.replacements.is_some() && self.batch.is_none() {
            return;
        }
        if let Some(entries) = &mut self.batch {
            if self.occupied {
                for (index, stored) in entries.iter_mut().rev() {
                    *stored = Some(elements.remove(*index));
                }
            } else {
                for (index, stored) in entries {
                    elements.insert(*index, stored.take().unwrap());
                }
            }
            if self.occupied {
                // 重做先移除矢量，再交换剩余效果。
                self.swap_replacements(elements);
            }
            self.occupied = !self.occupied;
            return;
        }
        if raster_change {
            return;
        }
        if let Some(previous) = self.serial {
            self.serial = Some(std::mem::replace(serial_count, previous));
        }
        if let Some(angle) = self.rotation {
            self.rotation = Some(std::mem::replace(&mut elements[self.index].rotation, angle));
            return;
        }
        if let Some(style) = self.style {
            self.style = Some(elements[self.index].apply_style(style));
            return;
        }
        match (self.occupied, self.stored.take()) {
            (true, None) => {
                self.stored = Some(elements.remove(self.index));
                self.occupied = false;
            }
            (false, Some(stroke)) => {
                elements.insert(self.index, stroke);
                self.occupied = true;
            }
            (true, Some(stroke)) => {
                self.stored = Some(std::mem::replace(&mut elements[self.index], stroke));
            }
            (false, None) => unreachable!("history entry has neither document nor stored content"),
        }
    }
}

#[derive(Default)]
pub(crate) struct History {
    entries: VecDeque<Entry>,
    cursor: usize,
}
impl History {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// 返回丢弃重做分支及最旧记录时实际释放的点数。文档中的点不会随历史淘汰丢失。
    pub fn record(&mut self, entry: Entry) -> usize {
        let mut released = self
            .entries
            .drain(self.cursor..)
            .map(|e| e.retained_points())
            .sum();
        self.entries.push_back(entry);
        if self.entries.len() > LIMIT {
            released += self.entries.pop_front().unwrap().retained_points();
        }
        self.cursor = self.entries.len();
        released
    }
    pub fn undo(
        &mut self,
        elements: &mut Vec<Stroke>,
        serial_count: &mut u32,
        raster: &mut Option<crate::erase_layers::Layers>,
        crop: &mut Option<crate::editor::CropView>,
        logical_size: &mut Option<crate::crop_geometry::Size>,
    ) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.entries[self.cursor].toggle(elements, serial_count, raster, crop, logical_size);
        true
    }
    pub fn redo(
        &mut self,
        elements: &mut Vec<Stroke>,
        serial_count: &mut u32,
        raster: &mut Option<crate::erase_layers::Layers>,
        crop: &mut Option<crate::editor::CropView>,
        logical_size: &mut Option<crate::crop_geometry::Size>,
    ) -> bool {
        if self.cursor == self.entries.len() {
            return false;
        }
        self.entries[self.cursor].toggle(elements, serial_count, raster, crop, logical_size);
        self.cursor += 1;
        true
    }
}
