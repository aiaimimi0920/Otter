//! 单次指针手势内展开 Win32 的 32 位毫秒消息时间；不使用消息处理延迟估计速度。
#[derive(Default)]
pub struct MessageClock {
    last: Option<u32>,
    elapsed: u64,
}
impl MessageClock {
    pub fn start(&mut self, tick: u32) -> f64 {
        self.last = Some(tick);
        self.elapsed = 0;
        0.0
    }
    pub fn advance(&mut self, tick: u32) -> f64 {
        let Some(last) = self.last else {
            return self.start(tick);
        };
        let delta = tick.wrapping_sub(last);
        // 单次手势不允许跨越半个计数周期；倒退/陈旧消息不推进时序。
        if delta <= i32::MAX as u32 {
            self.elapsed += delta as u64;
            self.last = Some(tick);
        }
        self.elapsed as f64
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wraps_without_jumping_and_rejects_out_of_order_ticks() {
        let mut clock = MessageClock::default();
        assert_eq!(clock.start(u32::MAX - 5), 0.0);
        assert_eq!(clock.advance(4), 10.0);
        assert_eq!(clock.advance(4), 10.0);
        assert_eq!(clock.advance(2), 10.0);
        assert_eq!(clock.advance(14), 20.0);
        assert_eq!(clock.start(123), 0.0);
        assert_eq!(clock.advance(128), 5.0);
    }
}
