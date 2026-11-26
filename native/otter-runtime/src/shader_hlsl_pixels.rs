//! Explicit straight RGBA-to-PBGRA adaptation; no reuse of legacy WebGL unpremultiplication or ownership of PNG/commit transactions.
use super::*;
impl Frame {
    /// Native-display bridging only. PNG still uses the original straight frame, without reconstructing lossy original channels from PBGRA.
    pub fn pbgra(&self, stopped: &impl Fn() -> bool, deadline: Instant) -> Result<Pixels> {
        check(stopped, deadline)?;
        let count = contract::image_bytes(self.width, self.height)?;
        if count != self.rgba.len() {
            return Err(Error::Layout);
        }
        // Performance audit optimization:
        // Reserve capacity without pre-zeroing 33 MiB (`bytes.resize(count, 0)`) and delegate row conversion to
        // `otter_core::pixel_alpha::premultiply_rgba_to_pbgra` (compiled under `opt-level = 3` with opaque/transparent fast paths).
        let mut lease = crate::pixels::reserve(count).ok_or(Error::Budget)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(count).map_err(|_| Error::Budget)?;
        if !lease.grow_to(bytes.capacity()) {
            return Err(Error::Budget);
        }
        let stride = self.width as usize * 4;
        for source in self.rgba.chunks_exact(stride) {
            check(stopped, deadline)?;
            let start = bytes.len();
            bytes.resize(start + stride, 0);
            crate::pixel_alpha::premultiply_rgba_to_pbgra(source, &mut bytes[start..]);
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
