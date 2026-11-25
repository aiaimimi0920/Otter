//! 明确的 straight RGBA→PBGRA 适配；不复用旧 WebGL 反预乘，也不拥有 PNG/提交事务。
use super::*;
impl Frame {
    /// 只供原生显示桥接。原 straight 帧仍用于 PNG，不从 PBGRA 反推有损的原通道。
    pub fn pbgra(&self, stopped: &impl Fn() -> bool, deadline: Instant) -> Result<Pixels> {
        check(stopped, deadline)?;
        let count = contract::image_bytes(self.width, self.height)?;
        if count != self.rgba.len() {
            return Err(Error::Layout);
        }
        let (mut bytes, lease) = contract::allocate(count)?;
        for (source, target) in self
            .rgba
            .chunks_exact(self.width as usize * 4)
            .zip(bytes.chunks_exact_mut(self.width as usize * 4))
        {
            check(stopped, deadline)?;
            for (source, target) in source.chunks_exact(4).zip(target.chunks_exact_mut(4)) {
                let alpha = u16::from(source[3]);
                let premultiply = |c| ((u16::from(c) * alpha + 127) / 255) as u8;
                target.copy_from_slice(&[
                    premultiply(source[2]),
                    premultiply(source[1]),
                    premultiply(source[0]),
                    source[3],
                ]);
            }
        }
        check(stopped, deadline)?;
        Ok(Pixels::from_reserved(bytes, lease))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn straight_rgba_bgra_swizzle_premultiplication_and_transparent_rgb_are_explicit() {
        let frame = Frame {
            width: 3,
            height: 1,
            rgba: vec![255, 128, 64, 128, 16, 255, 0, 0, 1, 2, 3, 255].into(),
            backend: SelectedBackend::D3d12,
        };
        let deadline = Instant::now() + std::time::Duration::from_secs(1);
        assert_eq!(
            &*frame.pbgra(&|| false, deadline).unwrap(),
            &[32, 64, 128, 128, 0, 0, 0, 0, 3, 2, 1, 255]
        );
        assert_eq!(&frame.rgba[4..8], &[16, 255, 0, 0]);
        assert_eq!(
            frame.pbgra(&|| true, deadline).unwrap_err(),
            Error::Cancelled
        );
        assert_eq!(
            frame.pbgra(&|| false, Instant::now()).unwrap_err(),
            Error::TimedOut
        );
        let invalid = Frame { width: 2, ..frame };
        assert_eq!(
            invalid.pbgra(&|| false, deadline).unwrap_err(),
            Error::Layout
        );
    }
    #[test]
    fn no_pixel_allocation_occurs_after_budget_exhaustion() {
        let budget = crate::byte_budget::Budget::new(4);
        let lease = budget.reserve(4).unwrap();
        assert!(matches!(
            contract::allocate_with(4, |n| budget.reserve(n)),
            Err(Error::Budget)
        ));
        drop(lease);
        assert!(contract::allocate_with(4, |n| budget.reserve(n)).is_ok());
    }
}
