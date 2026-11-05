//! 与 Wall v1 同义的最小 RTT 时钟估计；只用单调时间推进，旧 daemon 时钟不继承场景。
use crate::tile_wire::Timing;
use std::time::{Duration, Instant};
#[derive(Clone)]
struct Sample {
    at: Instant,
    server: f64,
    error: f64,
}
#[derive(Clone, Default)]
pub(crate) struct Clock {
    id: String,
    samples: Vec<Sample>,
}
impl Clock {
    pub fn observe(&mut self, t: &Timing, sent: Instant, received: Instant) -> bool {
        let changed = !self.id.is_empty() && self.id != t.clock_id;
        if self.id != t.clock_id {
            self.samples.clear();
            self.id.clone_from(&t.clock_id);
        }
        self.samples
            .retain(|s| received.saturating_duration_since(s.at) <= Duration::from_secs(15));
        let rtt = received.saturating_duration_since(sent).as_secs_f64() * 1000.;
        if rtt <= 1000. {
            self.samples.push(Sample {
                at: received,
                server: t.server_time_ms as f64 + rtt / 2.,
                error: rtt / 2.,
            });
            if self.samples.len() > 8 {
                self.samples.remove(0);
            }
        }
        changed
    }
    pub fn estimate(&self, now: Instant) -> Option<(u64, u64)> {
        let s = self
            .samples
            .iter()
            .filter(|s| now.saturating_duration_since(s.at) <= Duration::from_secs(15))
            .min_by(|a, b| a.error.total_cmp(&b.error))?;
        let age = now.saturating_duration_since(s.at).as_secs_f64() * 1000.;
        let server = s.server + age;
        Some((
            server.floor() as u64,
            (s.error + age * 0.0001 + 1.).ceil() as u64,
        ))
    }
    pub fn permits(&self, at: u64, now: Instant) -> bool {
        self.estimate(now)
            .is_some_and(|(server, error)| server.saturating_sub(error) >= at)
    }
}
