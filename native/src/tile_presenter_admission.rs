//! 本地paint不是服务端applied确认；只在原串行heartbeat成功后开放当前布局的未发输入。
//! 这是准入围栏而非授权/第二队列；不持像素，不延长期限，不重放未知写。
use super::{Key, State};

pub(crate) struct Acknowledged {
    generation: u64,
    key: Key,
}
pub(crate) struct Heartbeat {
    generation: u64,
    key: Option<Key>,
    applied: Option<u64>,
}
impl Heartbeat {
    /// 与生成heartbeat正文共用同一次Shared借用；网络等待期间不持该锁。
    pub(crate) fn capture(state: &mut State, key: Option<&Key>, applied: Option<u64>) -> Self {
        // 减权必须先于HTTP：服务可能已清applied但回复尚在途；后续paint不能沿用旧ACK。
        // 过期key的调用不撤后继owner；同布局正常续期也不制造无必要的输入停顿。
        if state.desired.as_ref() == key
            && (key.is_none_or(|key| applied != Some(key.revision))
                || state.black
                || state.frozen
                || state.identification.blocked())
        {
            state.input_applied = None;
        }
        Self {
            generation: state.generation,
            key: key.cloned(),
            applied,
        }
    }
    /// 仅原presenter在HTTP/严格ACK校验成功之后调用；同presenter没有并行心跳。
    pub(crate) fn acknowledge(self, state: &mut State) {
        if state.stopped
            || state.expired
            || state.generation != self.generation
            || state.desired != self.key
        {
            return;
        }
        state.input_applied = self
            .key
            .filter(|key| {
                self.applied == Some(key.revision)
                    && !state.black
                    && !state.frozen
                    && !state.identification.blocked()
            })
            .map(|key| Acknowledged {
                generation: self.generation,
                key,
            });
    }
}
pub(crate) fn permits(state: &State, key: &Key) -> bool {
    state.input_applied.as_ref().is_some_and(|ack| {
        ack.generation == state.generation && &ack.key == key && state.desired.as_ref() == Some(key)
    })
}

#[cfg(test)]
#[path = "tile_presenter_admission_tests.rs"]
mod tests;
