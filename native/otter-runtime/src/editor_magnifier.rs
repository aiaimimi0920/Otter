//! Lens source offset is relative to the destination size, so translating,
//! scaling or serializing the box also maps its sample center without drift.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lens {
    pub offset: Point,
    pub percent: u16,
}
impl Default for Lens {
    fn default() -> Self {
        Self {
            offset: Point::default(),
            percent: 250,
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
    fn locate_source(&mut self, p: Point) -> Result<(), EditError> {
        let [a, b] = self.points.as_slice() else {
            return Err(EditError::InvalidPoint);
        };
        let c = crate::rotation::center(&self.points);
        let offset = Point {
            x: (p.x - c.x) / (a.x - b.x).abs(),
            y: (p.y - c.y) / (a.y - b.y).abs(),
        };
        validate(offset)?;
        self.lens.as_mut().ok_or(EditError::InvalidPoint)?.offset = offset;
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
            _ => return Ok(None),
        }
        Ok(Some(next))
    }
}
impl Editor {
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
    pub(crate) fn restore_lens(&mut self, offset: Point, percent: u16) -> Result<(), EditError> {
        validate(offset)?;
        let s = self.elements.last_mut().ok_or(EditError::InvalidPoint)?;
        if s.tool != Tool::Magnifier || !(150..=800).contains(&percent) {
            return Err(EditError::InvalidPoint);
        }
        s.lens = Some(Lens { offset, percent });
        validate(s.lens_handles().ok_or(EditError::InvalidPoint)?[0])
    }
}
