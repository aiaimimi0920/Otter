//! 外部 Live 源的显式 shell 召回状态；无 OS 调用、锁、分配或全局输入监听。
//! 宿主只可在验证过的非注入 shell 输入上 record；前台变化本身不产生授权。
pub const INTENT_LIFETIME_MS: u64 = 2000;

#[derive(Clone, Copy, Debug)]
pub struct Intent {
    sequence: u64,
    issued_ms: u64,
    origin: usize,
}
impl Intent {
    fn valid_at(self, now_ms: u64) -> bool {
        now_ms
            .checked_sub(self.issued_ms)
            .is_some_and(|age| age <= INTENT_LIFETIME_MS)
    }
}

/// UI owner 串行记录和取消；调用方必须每次读取当前快照，不缓存已取消的 Intent。
#[derive(Default)]
pub struct IntentTracker {
    sequence: u64,
    current: Option<Intent>,
}
impl IntentTracker {
    /// now_ms 来自同一单调时钟；origin 为输入发生前的前台 HWND，不是待恢复目标。
    /// 序号耗尽时保守撤销授权，绝不回绕或用时间戳充当序号。
    pub fn record(&mut self, now_ms: u64, origin: usize) -> bool {
        self.current = None;
        let Some(sequence) = self.sequence.checked_add(1) else {
            return false;
        };
        self.sequence = sequence;
        self.current = Some(Intent {
            sequence,
            issued_ms: now_ms,
            origin,
        });
        true
    }
    pub fn cancel(&mut self) {
        self.current = None;
    }
    pub fn snapshot(&self, now_ms: u64) -> Option<Intent> {
        self.current.filter(|intent| intent.valid_at(now_ms))
    }
}

/// 每个共享可见性 owner 一份，而不是每个帧消费者一份。
/// 用户召回只恢复可见性；调用方不能因此释放采集/输入 lease。
pub struct Activation {
    consumed: u64,
    foreground: usize,
}
impl Activation {
    /// 开始逻辑隐藏时吸收已有意图，避免隐藏前的手势立即将源恢复。
    pub fn new(current: Option<Intent>, foreground: usize) -> Self {
        Self {
            consumed: current.map_or(0, |intent| intent.sequence),
            foreground,
        }
    }
    pub fn observe(
        &mut self,
        foreground: usize,
        source: usize,
        iconic: bool,
        current: Option<Intent>,
        now_ms: u64,
    ) -> bool {
        let previous = self.foreground;
        self.foreground = foreground;
        let Some(intent) = current else {
            return false;
        };
        // 已在前台的透明源被点击任务栏可能变为最小化；其他最小化源不能消费该手势。
        let transition = source != 0
            && ((iconic && intent.origin == source)
                || (!iconic && foreground == source && previous != source));
        if !transition || intent.sequence <= self.consumed {
            return false;
        }
        self.consumed = intent.sequence;
        intent.valid_at(now_ms)
    }
}

#[cfg(test)]
#[path = "live_reclaim_tests.rs"]
mod tests;
