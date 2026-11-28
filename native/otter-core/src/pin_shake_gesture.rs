//! Time-windowed horizontal extrema, independent of mouse report frequency.
//! Only the drag owner feeds this state; programmatic moves never arm it.
#[derive(Default)]
pub struct Shake {
    drag: Option<Drag>,
    fired: Option<u32>,
}
struct Drag {
    dpi: u32,
    origin: i32,
    extreme: i32,
    time: u32,
    sign: i64,
    turns: std::collections::VecDeque<u32>,
}
impl Shake {
    pub fn begin(&mut self, x: i32, ms: u32, dpi: u32) {
        self.drag = (dpi != 0).then(|| Drag {
            dpi,
            origin: x,
            extreme: x,
            time: ms,
            sign: 0,
            turns: Default::default(),
        });
    }
    pub fn end(&mut self) {
        self.drag = None;
    }
    pub fn sample(&mut self, x: i32, ms: u32) -> bool {
        let Some(d) = &mut self.drag else {
            return false;
        };
        if self.fired.is_some_and(|t| ms.wrapping_sub(t) < 500) {
            d.origin = x;
            d.extreme = x;
            d.time = ms;
            d.sign = 0;
            d.turns.clear();
            return false;
        }
        let delta = i64::from(x) - i64::from(d.extreme);
        let meaningful = |px: i64| px.abs() * 96 >= 12 * i64::from(d.dpi);
        if d.sign == 0 {
            if meaningful(delta) {
                d.sign = delta.signum();
                d.extreme = x;
                d.time = ms;
            }
            return false;
        }
        if delta.signum() == d.sign {
            d.extreme = x;
            d.time = ms;
            return false;
        }
        if !meaningful(delta) {
            return false;
        }
        let span = i64::from(d.extreme) - i64::from(d.origin);
        if meaningful(span) && span.abs() * 96 <= 420 * i64::from(d.dpi) {
            while d.turns.front().is_some_and(|&t| ms.wrapping_sub(t) > 700) {
                d.turns.pop_front();
            }
            if ms.wrapping_sub(d.time) <= 700 {
                d.turns.push_back(d.time);
            }
        } else {
            d.turns.clear();
        }
        d.origin = d.extreme;
        d.extreme = x;
        d.sign = delta.signum();
        d.time = ms;
        if d.turns.len() >= 2 && delta.abs() * 96 <= 420 * i64::from(d.dpi) {
            self.fired = Some(ms);
            d.turns.clear();
            d.sign = 0;
            return true;
        }
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slow_oversized_and_subthreshold_motion_never_fire_and_clock_wrap_is_safe() {
        for (amplitude, dt) in [(11, 10u32), (425, 10), (100, 750)] {
            let mut g = Shake::default();
            g.begin(0, 0, 96);
            for i in 1..16 {
                assert!(!g.sample(if i % 2 == 1 { amplitude } else { 0 }, i * dt));
            }
        }
        let mut g = Shake::default();
        let t = u32::MAX - 100;
        g.begin(0, t, 96);
        let hits = (1..=5)
            .filter(|i| g.sample(if i % 2 == 1 { 75 } else { 0 }, t.wrapping_add(i * 40)))
            .count();
        assert_eq!(hits, 1);
    }
    #[test]
    fn identical_intent_at_125_and_1000_hz_cooldown_and_noise() {
        for dt in [1, 8] {
            let mut g = Shake::default();
            g.begin(0, 0, 144);
            let mut hits = 0;
            for t in (0..600).step_by(dt) {
                let phase = t % 120;
                let x = if phase < 60 { phase } else { 120 - phase };
                hits += g.sample(x as i32, t) as usize;
            }
            assert_eq!(hits, 1);
            g.end();
            assert!(!g.sample(160, 3000));
            g.begin(0, 3000, 96);
            for x in 0..3000 {
                assert!(!g.sample(x, 3000 + x as u32));
            }
        }
    }
}
