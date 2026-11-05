//! Canvas 的跨重绘创建准入；保存首个未确认关联错误和最近一次未关联失败凭证，不持有 GPU 资源。
use crate::desktop::composition_creation;
use windows::core::{Error, Result};

#[derive(Default)]
pub(super) struct Admission {
    blocked: Option<Error>,
    unassociated_failure: bool,
}

impl Admission {
    pub(super) fn blocked(&self) -> bool {
        self.blocked.is_some()
    }
    pub(super) fn retryable_failure(&self) -> bool {
        self.unassociated_failure && !self.blocked()
    }
    pub(super) fn attempt<T>(
        &mut self,
        create: impl FnOnce() -> composition_creation::Result<T>,
    ) -> Result<T> {
        if let Some(error) = &self.blocked {
            return Err(error.clone());
        }
        // 新候选开始前撤销旧凭证；只有类型化的未关联失败可重新签发，成功不能留下解锁标记。
        self.unassociated_failure = false;
        match create() {
            Ok(value) => Ok(value),
            Err(failure) => {
                let denied = !failure.fallback_allowed();
                self.unassociated_failure = !denied;
                let error = failure.into_error();
                if denied {
                    self.blocked = Some(error.clone());
                }
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::E_FAIL;
    #[test]
    fn denied_association_is_sticky_but_preparation_failure_can_retry() {
        let mut gate = Admission::default();
        let error = gate
            .attempt::<()>(|| Err(Error::new(E_FAIL, "prepare").into()))
            .unwrap_err();
        assert_eq!(error.code(), E_FAIL);
        assert!(!gate.blocked());
        assert!(gate.retryable_failure());
        assert_eq!(gate.attempt(|| Ok(7)).unwrap(), 7);
        assert!(!gate.retryable_failure());
        gate.attempt::<()>(|| {
            Err(composition_creation::Failure::attached(Error::new(
                E_FAIL, "attach",
            )))
        })
        .unwrap_err();
        assert!(gate.blocked());
        assert!(!gate.retryable_failure());
        for _ in 0..3 {
            let error = gate
                .attempt::<()>(|| panic!("blocked creation retried"))
                .unwrap_err();
            assert_eq!(error.code(), E_FAIL);
            assert!(error.message().contains("attach"));
        }
    }
}
