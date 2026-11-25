//! Worker-exclusive capture-source contract; prepare movement first, then append outcomes decide physical-reference-region commit.
use super::*;
pub trait CaptureSource: Send {
    fn can_scroll(&self) -> bool {
        false
    }
    fn can_move(&self) -> bool {
        false
    }
    fn capture(&mut self, mode: CaptureMode, cancel: &AtomicBool) -> Result<Frame, String>;
    /// Prepare only candidate frames/regions without overwriting accepted references; errors still receive finish_move(false).
    fn prepare_move(
        &mut self,
        _movement: (i32, i32),
        _cancel: &AtomicBool,
    ) -> Result<Frame, String> {
        Err("Viewfinder movement unavailable".into())
    }
    /// Only commit/discard prepared internal state; no new external actions, blocking, or failure.
    /// Always false on cancellation; Drop must also release unfinished candidates for abnormal worker exit.
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
