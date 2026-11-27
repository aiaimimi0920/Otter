//! Volatile provenance of a successfully delivered ordinary region. No pixels or window titles.
use crate::capture_loupe_layout::Bounds;

#[derive(Clone, Debug)]
pub struct Last {
    rect: Bounds,
    topology: Vec<Bounds>,
}
fn clip(a: Bounds, b: Bounds) -> Option<Bounds> {
    let r = [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ];
    (r[0] < r[2] && r[1] < r[3]).then_some(r)
}
pub fn covered(rect: Bounds, monitors: &[Bounds]) -> bool {
    if rect[0] >= rect[2] || rect[1] >= rect[3] || monitors.len() > 64 {
        return false;
    }
    let tiles: Vec<_> = monitors.iter().filter_map(|r| clip(rect, *r)).collect();
    let mut xs = vec![rect[0], rect[2]];
    for r in &tiles {
        xs.extend([r[0], r[2]]);
    }
    xs.sort_unstable();
    xs.dedup();
    xs.windows(2).all(|span| {
        let mut intervals: Vec<_> = tiles
            .iter()
            .filter(|r| r[0] <= span[0] && r[2] >= span[1])
            .map(|r| (r[1], r[3]))
            .collect();
        intervals.sort_unstable();
        let mut end = rect[1];
        for (lo, hi) in intervals {
            if lo > end {
                return false;
            }
            end = end.max(hi);
        }
        end >= rect[3]
    })
}
impl Last {
    pub fn new(rect: Bounds, mut topology: Vec<Bounds>) -> Option<Self> {
        if topology.is_empty()
            || topology.len() > 64
            || !topology.iter().any(|m| clip(rect, *m).is_some())
        {
            return None;
        }
        topology.sort_unstable();
        Some(Self { rect, topology })
    }
    pub fn restore(&self, monitors: &[Bounds]) -> Option<Bounds> {
        if monitors.is_empty() || monitors.len() > 64 {
            return None;
        }
        if self.topology == monitors && covered(self.rect, monitors) {
            return Some(self.rect);
        }
        let pieces: Vec<_> = monitors
            .iter()
            .filter_map(|r| clip(self.rect, *r))
            .collect();
        let union = pieces.iter().copied().reduce(|a, b| {
            [
                a[0].min(b[0]),
                a[1].min(b[1]),
                a[2].max(b[2]),
                a[3].max(b[3]),
            ]
        })?;
        if covered(union, monitors) {
            return Some(union);
        }
        // A rectangular selection cannot represent a hole. Use the largest
        // contiguous clipped monitor rectangle rather than filling a missing display.
        pieces.into_iter().max_by_key(|r| {
            (
                (i64::from(r[2]) - i64::from(r[0])) as u64
                    * (i64::from(r[3]) - i64::from(r[1])) as u64,
                *r,
            )
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restores_contiguous_spans_but_never_a_hole_after_topology_change() {
        let left = [-100, 0, 0, 100];
        let right = [0, 0, 100, 100];
        let rect = [-80, 20, 80, 80];
        let last = Last::new(rect, vec![left, right]).unwrap();
        assert_eq!(last.restore(&[left, right]), Some(rect));
        assert_eq!(last.restore(&[left]), Some([-80, 20, 0, 80]));
        assert_eq!(last.restore(&[[200, 10, 300, 150]]), None);
        assert!(!covered(rect, &[left, [0, 60, 100, 160]]));
        assert_eq!(
            last.restore(&[left, [0, 60, 100, 160]]),
            Some([-80, 20, 0, 80])
        );
    }
}
