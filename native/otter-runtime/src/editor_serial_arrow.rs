//! The circle and the callout endpoint have independent handles. Keeping the
//! endpoint in circle-local radii lets translation, rotation and zoom share the
//! ordinary two-point serial geometry without changing its diameter.
use super::*;
impl Stroke {
    fn serial_circle(&self) -> Option<(Point, f32)> {
        if self.tool != Tool::Serial || self.points.len() != 2 {
            return None;
        }
        let r = (self.points[1].x - self.points[0].x) * 0.5;
        (r > 0.).then(|| (crate::rotation::center(&self.points), r))
    }
    pub fn serial_handles(&self) -> Option<[Point; 2]> {
        let (c, r) = self.serial_circle()?;
        let u = self.serial_arrow_tip?;
        Some([
            c,
            Point {
                x: c.x + u.x * r,
                y: c.y + u.y * r,
            },
        ])
    }
    pub fn serial_line(&self) -> Option<[Point; 2]> {
        let [center, tip] = self.serial_handles()?;
        let (_, r) = self.serial_circle()?;
        let (dx, dy) = (tip.x - center.x, tip.y - center.y);
        let len = dx.hypot(dy);
        (len > 0.).then(|| {
            [
                Point {
                    x: center.x + dx / len * r,
                    y: center.y + dy / len * r,
                },
                tip,
            ]
        })
    }
    pub(super) fn set_tip(&mut self, p: Point) -> Result<(), EditError> {
        validate(p)?;
        let (c, r) = self.serial_circle().ok_or(EditError::InvalidPoint)?;
        let relative = Point {
            x: (p.x - c.x) / r,
            y: (p.y - c.y) / r,
        };
        validate(relative)?;
        self.serial_arrow_tip = (relative != Point::default()).then_some(relative);
        Ok(())
    }
    pub(super) fn extend_serial(&mut self, p: Point, force: bool) -> Result<bool, EditError> {
        let (c, r) = self.serial_circle().ok_or(EditError::InvalidPoint)?;
        let before = self.serial_arrow_tip;
        if force || (p.x - c.x).hypot(p.y - c.y) >= 1.5 * r {
            self.set_tip(p)?;
        } else {
            self.serial_arrow_tip = None;
        }
        Ok(before != self.serial_arrow_tip)
    }
    pub fn preview_serial_handle(&self, index: usize, p: Point) -> Result<Option<Self>, EditError> {
        let Some([center, tip]) = self.serial_handles() else {
            return Ok(None);
        };
        validate(p)?;
        let mut next = self.clone();
        match index {
            0 => {
                let world = crate::rotation::rotate(p, center, self.rotation);
                let world_tip = crate::rotation::rotate(tip, center, self.rotation);
                for pt in &mut next.points {
                    pt.x += world.x - center.x;
                    pt.y += world.y - center.y;
                    validate(*pt)?;
                }
                next.set_tip(crate::rotation::rotate(world_tip, world, -self.rotation))?;
            }
            1 => next.set_tip(p)?,
            _ => return Ok(None),
        }
        Ok(Some(next))
    }
}
impl Editor {
    pub(crate) fn restore_serial_tip(&mut self, relative: Point) -> Result<(), EditError> {
        validate(relative)?;
        let s = self.elements.last_mut().ok_or(EditError::InvalidPoint)?;
        if relative == Point::default() || s.serial_circle().is_none() {
            return Err(EditError::InvalidPoint);
        }
        s.serial_arrow_tip = Some(relative);
        validate(s.serial_handles().ok_or(EditError::InvalidPoint)?[1])
    }
    pub fn move_serial_handle(
        &mut self,
        index: usize,
        handle: usize,
        p: Point,
    ) -> Result<bool, EditError> {
        let Some(source) = self.elements.get(index) else {
            return Ok(false);
        };
        let Some(changed) = source.preview_serial_handle(handle, p)? else {
            return Ok(false);
        };
        self.commit_label_preview(index, changed)
    }
    pub(super) fn commit_label_preview(
        &mut self,
        index: usize,
        changed: Stroke,
    ) -> Result<bool, EditError> {
        let source = &self.elements[index];
        if self.draft.is_some()
            || source.tool == changed.tool
                && source.corner_radius == changed.corner_radius
                && source.points == changed.points
                && source.serial_arrow_tip == changed.serial_arrow_tip
                && source.lens == changed.lens
                && source.text == changed.text
        {
            return Ok(false);
        }
        if self.retained_points + changed.points.len() > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let old = std::mem::replace(&mut self.elements[index], changed);
        self.retained_points += old.points.len();
        self.retained_points -= self.history.record(Entry::replaced(index, old));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}
