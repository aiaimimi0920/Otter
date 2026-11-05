// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Owned low-resolution frame with explicit pixel origin and transparent sampling.
use rgb::RGBA8;
use std::borrow::Cow;

pub(super) struct Frame<'a> {
    pub width: usize,
    pub height: usize,
    pub origin: [i32; 2],
    pub pixels: Cow<'a, [RGBA8]>,
}
impl Frame<'_> {
    // Release any input borrow before reconstruction writes to the input image.
    // Reduced frames are already owned, so this transfers rather than copies them.
    pub fn into_owned(self) -> Frame<'static> {
        Frame {
            width: self.width,
            height: self.height,
            origin: self.origin,
            pixels: Cow::Owned(self.pixels.into_owned()),
        }
    }
    fn get(&self, x: i32, y: i32) -> RGBA8 {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            RGBA8::default()
        } else {
            self.pixels[y as usize * self.width + x as usize]
        }
    }
    pub fn sample(&self, point: [f64; 2]) -> RGBA8 {
        let x = point[0] - f64::from(self.origin[0]) - 0.5;
        let y = point[1] - f64::from(self.origin[1]) - 0.5;
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        let fx = x - x.floor();
        let fy = y - y.floor();
        let mut values = [0.0; 4];
        for (dy, wy) in [(0, 1.0 - fy), (1, fy)] {
            for (dx, wx) in [(0, 1.0 - fx), (1, fx)] {
                let p = self.get(ix + dx, iy + dy);
                for (sum, c) in values.iter_mut().zip([p.r, p.g, p.b, p.a]) {
                    *sum += f64::from(c) * wx * wy;
                }
            }
        }
        pixel(values)
    }
    pub fn resample(
        &self,
        left: [f64; 2],
        right: [f64; 2],
        scale: [f64; 2],
        center: [f64; 2],
    ) -> Self {
        let origin = left.map(|v| v.floor() as i32);
        let width = (right[0].ceil() - left[0].floor()) as usize;
        let height = (right[1].ceil() - left[1].floor()) as usize;
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                pixels.push(self.sample([
                    (f64::from(origin[0]) + x as f64 + 0.5) / scale[0] + center[0],
                    (f64::from(origin[1]) + y as f64 + 0.5) / scale[1] + center[1],
                ]));
            }
        }
        Self {
            width,
            height,
            origin,
            pixels: Cow::Owned(pixels),
        }
    }
    pub fn blur(self, sigma: f64, horizontal: bool) -> Self {
        if sigma <= 0.0 {
            return self;
        }
        let radius = (3.0 * sigma).ceil() as i32;
        let mut weights: Vec<_> = (-radius..=radius)
            .map(|x| (-f64::from(x * x) / (2.0 * sigma * sigma)).exp())
            .collect();
        let total: f64 = weights.iter().sum();
        for weight in &mut weights {
            *weight /= total;
        }
        let pad = radius + 1;
        let px = if horizontal { pad } else { 0 };
        let py = if horizontal { 0 } else { pad };
        let width = self.width + 2 * px as usize;
        let height = self.height + 2 * py as usize;
        let origin = [self.origin[0] - px, self.origin[1] - py];
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let mut values = [0.0; 4];
                for (offset, weight) in (-radius..=radius).zip(&weights) {
                    let p = self.get(
                        x as i32 - px + if horizontal { offset } else { 0 },
                        y as i32 - py + if horizontal { 0 } else { offset },
                    );
                    for (sum, c) in values.iter_mut().zip([p.r, p.g, p.b, p.a]) {
                        *sum += f64::from(c) * weight;
                    }
                }
                pixels.push(pixel(values));
            }
        }
        Self {
            width,
            height,
            origin,
            pixels: Cow::Owned(pixels),
        }
    }
}
fn pixel(values: [f64; 4]) -> RGBA8 {
    let [r, g, b, a] = values.map(|v| v.round().clamp(0.0, 255.0) as u8);
    RGBA8::new(r, g, b, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_reduction_borrows_input_then_transfers_owned_storage() {
        let input = vec![RGBA8::new(12, 24, 48, 128); 64 * 48];
        let frame = Frame {
            width: 64,
            height: 48,
            origin: [0, 0],
            pixels: Cow::Borrowed(&input),
        };
        assert_eq!(frame.pixels.as_ptr(), input.as_ptr());
        let reduced = frame.resample([0.0, 0.0], [32.0, 24.0], [0.5, 0.5], [0.0; 2]);
        assert!(matches!(reduced.pixels, Cow::Owned(_)));
        assert_eq!(reduced.pixels.len(), 32 * 24);
        let allocation = reduced.pixels.as_ptr();
        let owned = reduced.into_owned();
        assert_eq!(allocation, owned.pixels.as_ptr());
        assert!(owned.pixels.iter().all(|p| *p == input[0]));
    }
}
