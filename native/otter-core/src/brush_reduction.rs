//! 固定 128 段窗口的迭代 RDP；窗口锚定笔迹起点，追加尾部不重分割已完成窗口。
use crate::{paintbrush_samples::Disk, viewport::Point};
const WINDOW: usize = 129;
const EMPTY: Disk = Disk {
    point: Point { x: 0.0, y: 0.0 },
    radius: 0.0,
};

pub struct Reduced<I> {
    source: I,
    buffer: [Disk; WINDOW],
    keep: [bool; WINDOW],
    length: usize,
    next: usize,
    carry: Option<Disk>,
    finished: bool,
}

impl<I: Iterator<Item = Disk>> Reduced<I> {
    pub fn new(source: I) -> Self {
        Self {
            source,
            buffer: [EMPTY; WINDOW],
            keep: [false; WINDOW],
            length: 0,
            next: 0,
            carry: None,
            finished: false,
        }
    }
    fn refill(&mut self) {
        self.length = 0;
        self.keep.fill(false);
        self.next = usize::from(self.carry.is_some());
        if let Some(carry) = self.carry.take() {
            self.buffer[0] = carry;
            self.length = 1;
        }
        while self.length < WINDOW {
            let Some(disk) = self.source.next() else {
                self.finished = true;
                break;
            };
            self.buffer[self.length] = disk;
            self.length += 1;
        }
        if self.length == 0 {
            return;
        }
        self.carry = Some(self.buffer[self.length - 1]);
        self.keep[0] = true;
        self.keep[self.length - 1] = true;
        let mut stack = [(0usize, 0usize); WINDOW];
        stack[0] = (0, self.length - 1);
        let mut count = 1;
        while count > 0 {
            count -= 1;
            let (start, end) = stack[count];
            let a = self.buffer[start];
            let b = self.buffer[end];
            let dx = b.point.x - a.point.x;
            let dy = b.point.y - a.point.y;
            let squared = dx * dx + dy * dy;
            let mut largest = 0.18;
            let mut split = None;
            for i in start + 1..end {
                let p = self.buffer[i];
                let t = if squared > 0.0 {
                    ((p.point.x - a.point.x) * dx + (p.point.y - a.point.y) * dy) / squared
                } else {
                    0.0
                }
                .clamp(0.0, 1.0);
                let error = (p.point.x - a.point.x - t * dx).hypot(p.point.y - a.point.y - t * dy)
                    + (p.radius - a.radius - t * (b.radius - a.radius)).abs();
                if error > largest {
                    largest = error;
                    split = Some(i);
                }
            }
            if let Some(split) = split {
                self.keep[split] = true;
                stack[count] = (start, split);
                stack[count + 1] = (split, end);
                count += 2;
            }
        }
    }
}

impl<I: Iterator<Item = Disk>> Iterator for Reduced<I> {
    type Item = Disk;
    fn next(&mut self) -> Option<Disk> {
        loop {
            while self.next < self.length {
                let index = self.next;
                self.next += 1;
                if self.keep[index] {
                    return Some(self.buffer[index]);
                }
            }
            if self.finished {
                return None;
            }
            self.refill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maximum_straight_stroke_reduces_geometry_without_growing_window_storage() {
        let points: Vec<_> = (0..65_536)
            .map(|x| Point {
                x: x as f32,
                y: 0.0,
            })
            .collect();
        let raw = crate::paintbrush_samples::Samples::new(&points, 16.0, None)
            .unwrap()
            .count();
        let reduced =
            Reduced::new(crate::paintbrush_samples::Samples::new(&points, 16.0, None).unwrap())
                .count();
        assert!(reduced < raw / 20);
        assert!(std::mem::size_of::<Reduced<crate::paintbrush_samples::Samples<'_>>>() < 4096);
        println!(
            "native-paintbrush-reduction: points={} raw={} reduced={} iterator_bytes={}",
            points.len(),
            raw,
            reduced,
            std::mem::size_of::<Reduced<crate::paintbrush_samples::Samples<'_>>>()
        );
    }
    fn disk(x: f32, y: f32, radius: f32) -> Disk {
        Disk {
            point: Point { x, y },
            radius,
        }
    }
    #[test]
    fn collinear_disks_reduce_to_fixed_window_anchors_without_duplicates() {
        for length in [0, 1, 2, 128, 129, 130, 257, 258] {
            let input: Vec<_> = (0..length).map(|x| disk(x as f32, 0.0, 4.0)).collect();
            let result: Vec<_> = Reduced::new(input.iter().copied()).collect();
            let mut expected: Vec<_> = (0..length).step_by(128).map(|x| input[x]).collect();
            if let Some(&last) = input.last() {
                if expected.last() != Some(&last) {
                    expected.push(last);
                }
            }
            assert_eq!(result, expected, "length={length}");
        }
    }
    #[test]
    fn reversals_and_radius_changes_are_not_discarded() {
        for input in [
            vec![
                disk(0.0, 0.0, 1.0),
                disk(10.0, 0.0, 1.0),
                disk(0.0, 0.0, 1.0),
            ],
            vec![
                disk(0.0, 0.0, 1.0),
                disk(5.0, 0.0, 4.0),
                disk(10.0, 0.0, 1.0),
            ],
        ] {
            assert_eq!(
                Reduced::new(input.iter().copied()).collect::<Vec<_>>(),
                input
            );
        }
    }
    #[test]
    fn appending_tail_does_not_change_completed_window() {
        let input: Vec<_> = (0..400)
            .map(|x| {
                disk(
                    x as f32,
                    (x as f32 / 7.0).sin(),
                    1.0 + (x % 9) as f32 / 10.0,
                )
            })
            .collect();
        let before: Vec<_> = Reduced::new(input[..180].iter().copied())
            .filter(|p| p.point.x <= 128.0)
            .collect();
        let after: Vec<_> = Reduced::new(input.iter().copied())
            .filter(|p| p.point.x <= 128.0)
            .collect();
        assert_eq!(before, after);
    }
}
