//! 每条交换链最多记录16次原始Present状态变化；不重试、不改变提交资格、不含用户内容。
use std::cell::Cell;
use windows::core::HRESULT;

#[derive(Default)]
pub(super) struct Trace {
    last: Cell<Option<HRESULT>>,
    emitted: Cell<u8>,
}
impl Trace {
    pub(super) fn observe(&self, backend: &'static str, status: HRESULT) {
        if self.changed(status) {
            eprintln!(
                "native-present-status: backend={} hresult=0x{:08X} confirmed={}",
                backend,
                status.0 as u32,
                status == windows::Win32::Foundation::S_OK,
            );
        }
    }
    fn changed(&self, status: HRESULT) -> bool {
        if self.emitted.get() == 16 || self.last.get() == Some(status) {
            return false;
        }
        self.last.set(Some(status));
        self.emitted.set(self.emitted.get() + 1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::{E_FAIL, S_OK};
    #[test]
    fn exact_status_changes_are_bounded_and_do_not_reset_on_success() {
        let trace = Trace::default();
        for status in [S_OK, HRESULT(0x087A0001), E_FAIL, S_OK] {
            assert!(trace.changed(status));
            assert_eq!(trace.last.get(), Some(status));
            for _ in 0..1000 {
                assert!(!trace.changed(status));
            }
        }
        for value in 1..=12 {
            assert!(trace.changed(HRESULT(value)));
        }
        assert_eq!(trace.emitted.get(), 16);
        assert!(!trace.changed(E_FAIL));
        assert!(!trace.changed(S_OK));
        assert!(Trace::default().changed(E_FAIL));
    }
}
