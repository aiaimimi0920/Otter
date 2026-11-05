//! worker 独占的采集源契约；移动先准备，追加结果决定是否提交物理参考区域。
use super::*;
pub trait CaptureSource: Send {
    fn can_scroll(&self) -> bool {
        false
    }
    fn can_move(&self) -> bool {
        false
    }
    fn capture(&mut self, mode: CaptureMode, cancel: &AtomicBool) -> Result<Frame, String>;
    /// 仅准备候选帧/区域，不覆盖已接受参考；出错后仍会收到 finish_move(false)。
    fn prepare_move(
        &mut self,
        _movement: (i32, i32),
        _cancel: &AtomicBool,
    ) -> Result<Frame, String> {
        Err("Viewfinder movement unavailable".into())
    }
    /// 只提交/丢弃已准备的内部状态，不得执行新的外部动作、阻塞或失败。
    /// 取消时总是 false；Drop 还须释放未完成候选，以覆盖 worker 异常退出。
    fn finish_move(&mut self, _accepted: bool) {}
}
type Callback = Box<dyn FnMut(CaptureMode, &AtomicBool) -> Result<Frame, String> + Send>;
pub(super) struct CallbackSource {
    pub callback: Callback,
    pub scroll: bool,
}
impl CaptureSource for CallbackSource {
    fn can_scroll(&self) -> bool {
        self.scroll
    }
    fn capture(&mut self, mode: CaptureMode, cancel: &AtomicBool) -> Result<Frame, String> {
        (self.callback)(mode, cancel)
    }
}
