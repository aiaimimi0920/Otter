//! EDIT 内边界使用四角独立椭圆裁剪；所有临时 GDI region 在错误路径同样释放。
use super::*;
struct Region(HRGN);
impl Drop for Region {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}
unsafe fn region(value: HRGN) -> Result<Region> {
    if value.is_invalid() {
        Err(Error::from_win32())
    } else {
        Ok(Region(value))
    }
}
pub(super) unsafe fn apply(
    region_to_clip: HRGN,
    size: [i32; 2],
    style: crate::surface_textarea::BoxStyle,
    scale: f32,
) -> Result<()> {
    let b = style.border;
    let mut radii = [
        [b.left, b.top],
        [b.right, b.top],
        [b.right, b.bottom],
        [b.left, b.bottom],
    ]
    .map(|side| std::array::from_fn::<_, 2, _>(|a| (style.radius[a] - side[a]).max(0.) * scale));
    for radius in &mut radii {
        if radius.contains(&0.) {
            *radius = [0.; 2];
        }
    }
    let mut factor = 1_f32;
    for (axis, first, last) in [(0, 0, 1), (0, 3, 2), (1, 0, 3), (1, 1, 2)] {
        let sum = radii[first][axis] + radii[last][axis];
        if sum > 0. {
            factor = factor.min(size[axis].max(0) as f32 / sum);
        }
    }
    // 保留既有等宽圆角 region 的量化路径。
    if radii.iter().all(|radius| *radius == radii[0]) {
        let radius = radii[0].map(|r| (r * factor * 2.).round() as i32);
        if radius.iter().all(|r| *r > 0) {
            let round = region(CreateRoundRectRgn(
                0,
                0,
                size[0] + 1,
                size[1] + 1,
                radius[0],
                radius[1],
            ))?;
            if CombineRgn(
                Some(region_to_clip),
                Some(region_to_clip),
                Some(round.0),
                RGN_AND,
            ) == RGN_ERROR
            {
                return Err(Error::from_win32());
            }
        }
        return Ok(());
    }
    for (index, radius) in radii.into_iter().enumerate() {
        let [rx, ry] = radius.map(|r| (r * factor).round() as i32);
        if rx <= 0 || ry <= 0 {
            continue;
        }
        let right = index == 1 || index == 2;
        let bottom = index >= 2;
        let [cx, cy] = [
            if right { size[0] - rx } else { rx },
            if bottom { size[1] - ry } else { ry },
        ];
        let ellipse = region(CreateEllipticRgn(
            cx - rx,
            cy - ry,
            cx + rx + 1,
            cy + ry + 1,
        ))?;
        let horizontal = region(CreateRectRgn(
            if right { 0 } else { cx },
            0,
            if right { cx } else { size[0] },
            size[1],
        ))?;
        let vertical = region(CreateRectRgn(
            0,
            if bottom { 0 } else { cy },
            size[0],
            if bottom { cy } else { size[1] },
        ))?;
        for other in [horizontal.0, vertical.0] {
            if CombineRgn(Some(ellipse.0), Some(ellipse.0), Some(other), RGN_OR) == RGN_ERROR {
                return Err(Error::from_win32());
            }
        }
        if CombineRgn(
            Some(region_to_clip),
            Some(region_to_clip),
            Some(ellipse.0),
            RGN_AND,
        ) == RGN_ERROR
        {
            return Err(Error::from_win32());
        }
    }
    Ok(())
}
