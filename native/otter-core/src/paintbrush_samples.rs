//! Stroke-start/end radius sample streams; at most eight samples per segment, without prefix distances/full derived-point arrays.
use crate::{brush_dynamics, viewport::Point};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Disk {
    pub point: Point,
    pub radius: f32,
}

pub struct Samples<'a> {
    points: &'a [Point],
    weights: Option<&'a [f32]>,
    radius: f32,
    total: f32,
    taper: f32,
    distance: f32,
    segment: usize,
    step: usize,
    current: Option<Segment>,
}

#[derive(Clone, Copy)]
struct Segment {
    a: Point,
    b: Point,
    length: f32,
    divisions: usize,
    weights: Option<(f32, f32)>,
}

impl<'a> Samples<'a> {
    pub fn new(points: &'a [Point], width: f32, weights: Option<&'a [f32]>) -> Option<Self> {
        if points.is_empty()
            || points.len() > 65_536
            || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite())
        {
            return None;
        }
        let radius = if width.is_finite() {
            width.clamp(1.0, 999.0) / 2.0
        } else {
            8.0
        };
        let total = points
            .windows(2)
            .map(|p| distance(clamp(p[0]), clamp(p[1])))
            .sum::<f32>();
        Some(Self {
            points,
            weights: weights.filter(|w| w.len() == points.len()),
            radius,
            total,
            taper: (total * 0.28).min(10.0_f32.max(radius * 2.5)),
            distance: 0.0,
            segment: 1,
            step: 0,
            current: None,
        })
    }
}

impl Iterator for Samples<'_> {
    type Item = Disk;
    fn next(&mut self) -> Option<Disk> {
        if self.segment == usize::MAX {
            return None;
        }
        if self.total < 1.0 || self.points.len() == 1 {
            self.segment = usize::MAX;
            return Some(Disk {
                point: clamp(self.points[0]),
                radius: self.radius,
            });
        }
        if self.segment >= self.points.len() {
            return None;
        }
        // The endpoints, length, and pressure do not change within a segment.
        let Segment {
            a,
            b,
            length,
            divisions,
            weights,
        } = *self.current.get_or_insert_with(|| {
            let a = clamp(self.points[self.segment - 1]);
            let b = clamp(self.points[self.segment]);
            let length = distance(a, b);
            let divisions =
                if self.distance < self.taper || self.distance + length > self.total - self.taper {
                    8
                } else {
                    1
                };
            let weights = self.weights.map(|weights| {
                (
                    brush_dynamics::weight(weights[self.segment - 1]),
                    brush_dynamics::weight(weights[self.segment]),
                )
            });
            Segment {
                a,
                b,
                length,
                divisions,
                weights,
            }
        });
        let t = self.step as f32 / divisions as f32;
        let along = self.distance + t * length;
        let weight = weights.map_or(0.85, |(a, b)| a * (1.0 - t) + b * t);
        let start = 0.8 + 0.2 * (along / (self.taper * 0.45).max(1.0)).min(1.0);
        let remaining = ((self.total - along) / self.taper.max(1.0)).min(1.0);
        let finish = 0.08 + 0.92 * remaining * remaining * (3.0 - 2.0 * remaining);
        let disk = Disk {
            point: Point {
                x: a.x + (b.x - a.x) * t,
                y: a.y + (b.y - a.y) * t,
            },
            radius: self.radius * weight * start.min(finish),
        };
        if self.step == divisions {
            self.segment += 1;
            self.step = 1;
            self.distance += length;
            self.current = None;
        } else {
            self.step += 1;
        }
        Some(disk)
    }
}

fn clamp(p: Point) -> Point {
    Point {
        x: p.x.clamp(-100_000.0, 100_000.0),
        y: p.y.clamp(-100_000.0, 100_000.0),
    }
}
fn distance(a: Point, b: Point) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Native solid-color contract excludes color alpha; per-layer source-over compensation reaches requested opacity at the center.
pub fn bands(opacity: f32) -> [(f32, f32); 4] {
    let peak = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let mut previous = 0.0;
    std::array::from_fn(|index| {
        let t = (index + 1) as f32 / 4.0;
        let target = peak * t * t * (3.0 - 2.0 * t);
        let alpha = if previous < 1.0 {
            (target - previous) / (1.0 - previous)
        } else {
            0.0
        };
        previous = target;
        (1.0 - index as f32 * 0.12 / 3.0, alpha)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }
    #[test]
    fn taper_weights_and_sample_count_match_baseline() {
        let points = [p(0.0, 0.0), p(100.0, 0.0)];
        let samples: Vec<_> = Samples::new(&points, 16.0, None).unwrap().collect();
        assert_eq!(samples.len(), 9);
        assert!((samples[0].radius - 8.0 * 0.85 * 0.8).abs() < 0.0001);
        assert!((samples[8].radius - 8.0 * 0.85 * 0.08).abs() < 0.0001);
        assert!((samples[4].radius - 8.0 * 0.85).abs() < 0.0001);
        let weighted: Vec<_> = Samples::new(&points, 16.0, Some(&[0.4, 1.0]))
            .unwrap()
            .collect();
        assert!((weighted[4].radius - 8.0 * 0.7).abs() < 0.0001);
    }
    #[test]
    fn segment_transition_preserves_joins_and_uses_current_weights() {
        let points = [p(0.0, 0.0), p(30.0, 0.0), p(180.0, 0.0), p(220.0, 0.0)];
        let weights = [0.35, 0.8, 0.65, 1.0];
        let samples: Vec<_> = Samples::new(&points, 16.0, Some(&weights))
            .unwrap()
            .collect();
        assert_eq!(samples.len(), 18);
        assert!(samples
            .windows(2)
            .all(|pair| pair[0].point.x < pair[1].point.x));
        assert_eq!(samples[8].point, points[1]);
        assert_eq!(samples[9].point, points[2]);
        assert_eq!(samples[10].point.x, 185.0);
        assert!((samples[8].radius - 8.0 * weights[1]).abs() < 0.0001);
        assert!((samples[9].radius - 8.0 * weights[2]).abs() < 0.0001);
        assert!((samples[17].radius - 8.0 * weights[3] * 0.08).abs() < 0.0001);
    }
    #[test]
    fn duplicate_points_and_long_paths_remain_bounded() {
        assert_eq!(
            Samples::new(&[p(2.0, 3.0); 3], 16.0, None)
                .unwrap()
                .collect::<Vec<_>>(),
            vec![Disk {
                point: p(2.0, 3.0),
                radius: 8.0
            }]
        );
        let points: Vec<_> = (0..65_536).map(|x| p(x as f32, (x % 2) as f32)).collect();
        let count = Samples::new(&points, 16.0, None).unwrap().count();
        assert!(count <= 1 + 8 * (points.len() - 1));
        assert!(Samples::new(&[], 16.0, None).is_none());
        assert!(Samples::new(&[p(f32::NAN, 0.0)], 16.0, None).is_none());
    }
    #[test]
    fn compensated_bands_reconstruct_requested_center_opacity() {
        for opacity in [0.0, 0.25, 0.5, 1.0] {
            let mut combined = 0.0;
            for (radius, alpha) in bands(opacity) {
                assert!((0.88..=1.0).contains(&radius));
                combined = alpha + combined * (1.0 - alpha);
            }
            assert!((combined - opacity).abs() < 0.00001);
        }
    }
}
