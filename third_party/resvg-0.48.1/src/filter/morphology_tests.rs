//! Independent small-image rectangular oracle for the separable implementation.
use super::*;

#[test]
fn morphology_matches_naive_transparent_padding() {
    for width in 1..=5usize {
        for height in 1..=5usize {
            let input: Vec<_> = (0..width * height)
                .map(|i| {
                    let a = ((i * 71 + width * 31 + height * 17) % 256) as u8;
                    RGBA8::new(a / 2, a / 3, a, a)
                })
                .collect();
            for rx in 0..=6i32 {
                for ry in 0..=6i32 {
                    for operator in [MorphologyOperator::Erode, MorphologyOperator::Dilate] {
                        let mut expected = Vec::new();
                        for y in 0..height as i32 {
                            for x in 0..width as i32 {
                                let erode = operator == MorphologyOperator::Erode;
                                let mut result = if erode {
                                    RGBA8::new(255, 255, 255, 255)
                                } else {
                                    RGBA8::default()
                                };
                                for dy in -ry..=ry {
                                    for dx in -rx..=rx {
                                        let (sx, sy) = (x + dx, y + dy);
                                        let value = if sx < 0
                                            || sy < 0
                                            || sx >= width as i32
                                            || sy >= height as i32
                                        {
                                            RGBA8::default()
                                        } else {
                                            input[sy as usize * width + sx as usize]
                                        };
                                        for channel in 0..4 {
                                            let a = get(result, channel);
                                            let b = get(value, channel);
                                            set(
                                                &mut result,
                                                channel,
                                                if erode { a.min(b) } else { a.max(b) },
                                            );
                                        }
                                    }
                                }
                                expected.push(result);
                            }
                        }
                        let mut actual = input.clone();
                        apply(
                            operator,
                            rx as f32,
                            ry as f32,
                            ImageRefMut::new(width as u32, height as u32, &mut actual),
                        );
                        assert_eq!(
                            actual, expected,
                            "{width}x{height} radius {rx},{ry} {operator:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn morphology_large_radius_has_bounded_pixel_work() {
    for radius in [1., 256., 100000.] {
        let mut pixels = vec![RGBA8::new(32, 64, 128, 255); 1024 * 1024];
        let start = std::time::Instant::now();
        apply(
            MorphologyOperator::Dilate,
            radius,
            radius,
            ImageRefMut::new(1024, 1024, &mut pixels),
        );
        assert!(pixels.iter().all(|p| *p == RGBA8::new(32, 64, 128, 255)));
        println!(
            "morphology 1024x1024 radius={radius} elapsed_ms={}",
            start.elapsed().as_millis()
        );
    }
}
