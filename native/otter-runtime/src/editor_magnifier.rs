//! Lens source offset is relative to the destination size, so translating,
//! scaling or serializing the box also maps its sample center without drift.
use super::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LensShape {
    #[default]
    Rect,
    Rounded,
    Ellipse,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lens {
    pub offset: Point,
    pub percent: u16,
    pub shape: LensShape,
}
impl Default for Lens {
    fn default() -> Self {
        Self {
            offset: Point::default(),
            percent: 250,
            shape: LensShape::Rect,
        }
    }
}
impl Stroke {
    pub fn lens_handles(&self) -> Option<[Point; 2]> {
        let lens = self.lens?;
        let [a, b] = self.points.as_slice() else {
            return None;
        };
        let c = crate::rotation::center(&self.points);
        Some([
            Point {
                x: c.x + lens.offset.x * (a.x - b.x).abs(),
                y: c.y + lens.offset.y * (a.y - b.y).abs(),
            },
            c,
        ])
    }
    pub fn lens_edit_handles(&self) -> Option<[Point; 5]> {
        let [source, _] = self.lens_handles()?;
        let [a, b] = self.points.as_slice() else {
            return None;
        };
        let (left, right) = (a.x.min(b.x), a.x.max(b.x));
        let (top, bottom) = (a.y.min(b.y), a.y.max(b.y));
        Some([
            source,
            Point { x: left, y: top },
            Point { x: right, y: top },
            Point {
                x: right,
                y: bottom,
            },
            Point { x: left, y: bottom },
        ])
    }
    pub fn lens_source_box(&self) -> Option<(Point, Point)> {
        let lens = self.lens?;
        let [source, _] = self.lens_handles()?;
        let [a, b] = self.points.as_slice() else {
            return None;
        };
        let z = (f32::from(lens.percent) / 100.0).max(1.0);
        let hw = (a.x - b.x).abs() * 0.5 / z;
        let hh = (a.y - b.y).abs() * 0.5 / z;
        Some((
            Point {
                x: source.x - hw,
                y: source.y - hh,
            },
            Point {
                x: source.x + hw,
                y: source.y + hh,
            },
        ))
    }
    pub(crate) fn locate_source(&mut self, p: Point) -> Result<(), EditError> {
        let [a, b] = self.points.as_slice() else {
            return Err(EditError::InvalidPoint);
        };
        let w = (a.x - b.x).abs();
        let h = (a.y - b.y).abs();
        if w <= 0.0 || h <= 0.0 {
            return Err(EditError::InvalidPoint);
        }
        let c = crate::rotation::center(&self.points);
        let offset = Point {
            x: (p.x - c.x) / w,
            y: (p.y - c.y) / h,
        };
        validate(offset)?;
        self.lens.as_mut().ok_or(EditError::InvalidPoint)?.offset = offset;
        Ok(())
    }
    pub(crate) fn reanchor_translated_lens(&mut self, original: &Self) -> Result<(), EditError> {
        if original.tool == Tool::Magnifier
            && original.lens.is_some_and(|l| l.offset != Point::default())
        {
            if let Some([source, c]) = original.lens_handles() {
                let world_source = crate::rotation::rotate(source, c, original.rotation);
                let next_c = crate::rotation::center(&self.points);
                self.locate_source(crate::rotation::rotate(
                    world_source,
                    next_c,
                    -original.rotation,
                ))?;
            }
        }
        Ok(())
    }
    pub fn preview_lens_handle(&self, handle: usize, p: Point) -> Result<Option<Self>, EditError> {
        let Some([source, c]) = self.lens_handles() else {
            return Ok(None);
        };
        validate(p)?;
        let mut next = self.clone();
        match handle {
            0 => next.locate_source(p)?,
            1 => {
                let world = crate::rotation::rotate(p, c, self.rotation);
                let world_source = crate::rotation::rotate(source, c, self.rotation);
                for v in &mut next.points {
                    v.x += world.x - c.x;
                    v.y += world.y - c.y;
                    validate(*v)?;
                }
                next.locate_source(crate::rotation::rotate(world_source, world, -self.rotation))?;
            }
            2..=5 => {
                let [a, b] = self.points.as_slice() else {
                    return Ok(None);
                };
                let (mut left, mut right) = (a.x.min(b.x), a.x.max(b.x));
                let (mut top, mut bottom) = (a.y.min(b.y), a.y.max(b.y));
                let min_size = 16.0;
                match handle - 2 {
                    0 => {
                        left = p.x.min(right - min_size);
                        top = p.y.min(bottom - min_size);
                    }
                    1 => {
                        right = p.x.max(left + min_size);
                        top = p.y.min(bottom - min_size);
                    }
                    2 => {
                        right = p.x.max(left + min_size);
                        bottom = p.y.max(top + min_size);
                    }
                    _ => {
                        left = p.x.min(right - min_size);
                        bottom = p.y.max(top + min_size);
                    }
                }
                let p0 = Point { x: left, y: top };
                let p1 = Point {
                    x: right,
                    y: bottom,
                };
                validate(p0)?;
                validate(p1)?;
                let world_source = crate::rotation::rotate(source, c, self.rotation);
                let new_local_center = Point {
                    x: (left + right) * 0.5,
                    y: (top + bottom) * 0.5,
                };
                let world_center = crate::rotation::rotate(new_local_center, c, self.rotation);
                let shift = Point {
                    x: world_center.x - new_local_center.x,
                    y: world_center.y - new_local_center.y,
                };
                next.points[0] = Point {
                    x: p0.x + shift.x,
                    y: p0.y + shift.y,
                };
                next.points[1] = Point {
                    x: p1.x + shift.x,
                    y: p1.y + shift.y,
                };
                validate(next.points[0])?;
                validate(next.points[1])?;
                if self.lens.is_some_and(|l| l.offset != Point::default()) {
                    let next_c = crate::rotation::center(&next.points);
                    next.locate_source(crate::rotation::rotate(
                        world_source,
                        next_c,
                        -self.rotation,
                    ))?;
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(next))
    }
}
impl Editor {
    pub fn set_draft_lens(&mut self, lens: Lens) {
        if let Some(draft) = &mut self.draft {
            if draft.tool == Tool::Magnifier {
                draft.lens = Some(lens);
                if lens.shape == LensShape::Rounded && draft.corner_radius == 0.0 {
                    draft.corner_radius = 12.0;
                }
            }
        }
    }
    pub fn has_lens(&self) -> bool {
        self.elements
            .iter()
            .chain(self.draft.iter())
            .any(|s| s.tool == Tool::Magnifier)
    }
    pub fn move_lens_handle(
        &mut self,
        index: usize,
        handle: usize,
        p: Point,
    ) -> Result<bool, EditError> {
        let Some(s) = self.elements.get(index) else {
            return Ok(false);
        };
        let Some(next) = s.preview_lens_handle(handle, p)? else {
            return Ok(false);
        };
        self.commit_label_preview(index, next)
    }
    pub fn set_lens_zoom(&mut self, index: usize, percent: u16) -> Result<bool, EditError> {
        if !(150..=800).contains(&percent) {
            return Err(EditError::InvalidPoint);
        }
        let Some(s) = self.elements.get(index) else {
            return Ok(false);
        };
        if s.lens.ok_or(EditError::InvalidPoint)?.percent == percent {
            return Ok(false);
        }
        let mut next = s.clone();
        next.lens.as_mut().unwrap().percent = percent;
        self.commit_label_preview(index, next)
    }
    pub fn set_lens_shape(&mut self, index: usize, shape: LensShape) -> Result<bool, EditError> {
        let Some(s) = self.elements.get(index) else {
            return Ok(false);
        };
        if s.lens.ok_or(EditError::InvalidPoint)?.shape == shape {
            return Ok(false);
        }
        let mut next = s.clone();
        next.lens.as_mut().unwrap().shape = shape;
        if shape == LensShape::Rounded && next.corner_radius == 0.0 {
            next.corner_radius = 12.0;
        }
        self.commit_label_preview(index, next)
    }
    pub(crate) fn restore_lens(
        &mut self,
        offset: Point,
        percent: u16,
        shape: LensShape,
    ) -> Result<(), EditError> {
        validate(offset)?;
        let s = self.elements.last_mut().ok_or(EditError::InvalidPoint)?;
        if s.tool != Tool::Magnifier || !(150..=800).contains(&percent) {
            return Err(EditError::InvalidPoint);
        }
        s.lens = Some(Lens {
            offset,
            percent,
            shape,
        });
        validate(s.lens_handles().ok_or(EditError::InvalidPoint)?[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callout_magnifier_locks_world_source_across_translation_and_corner_resize() {
        let mut editor = Editor::default();
        editor
            .begin(Tool::Magnifier, Point { x: 100.0, y: 100.0 })
            .unwrap();
        editor.extend(Point { x: 200.0, y: 180.0 }).unwrap();
        editor.set_draft_lens(Lens::default());
        assert!(editor.commit());
        // Detach source target to (40, 50)
        assert!(editor
            .move_lens_handle(0, 0, Point { x: 40.0, y: 50.0 })
            .unwrap());
        assert_eq!(
            editor.elements()[0].lens_handles().unwrap()[0],
            Point { x: 40.0, y: 50.0 }
        );
        // Translating the callout lens keeps world source locked at (40, 50)
        assert!(editor.translate(0, Point { x: 60.0, y: 30.0 }).unwrap());
        let [src, center] = editor.elements()[0].lens_handles().unwrap();
        assert!((src.x - 40.0).abs() < 1e-3 && (src.y - 50.0).abs() < 1e-3);
        assert!((center.x - 210.0).abs() < 1e-3 && (center.y - 170.0).abs() < 1e-3);
        // Dragging bottom-right corner handle (index 4 -> handle 2+2=4) resizes lens while keeping source at (40, 50)
        assert!(editor
            .move_lens_handle(0, 4, Point { x: 300.0, y: 260.0 })
            .unwrap());
        let [src_after, _] = editor.elements()[0].lens_handles().unwrap();
        assert!((src_after.x - 40.0).abs() < 1e-3 && (src_after.y - 50.0).abs() < 1e-3);
    }
}
