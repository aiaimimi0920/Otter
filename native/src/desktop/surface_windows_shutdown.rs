//! 退出后的无网络结算门禁：回交已完成 owner，不显示、不重试，也不冒充持久化。
use super::*;
use crate::{desktop::composition::relay::sessions::Sessions, surface_live_binding::Lifecycle};

impl Windows {
    pub fn recovery_records(&self) -> Vec<crate::surface_recovery_record::Record> {
        use crate::surface_recovery_record::Record;
        let mut records = Vec::new();
        for window in &self.entries {
            let owner = window.lifecycle();
            records.push(Record::from_binding(
                owner.map_or_else(|| window.binding(), |owner| owner.binding()),
                Some(window.request().art()),
                owner.is_none_or(|owner| owner.uncertain()),
                owner.is_none(),
            ));
            if let Some(owner) = window.rejected_owner() {
                records.push(Record::from_binding(
                    owner.binding(),
                    None,
                    owner.uncertain(),
                    false,
                ));
            }
        }
        for failure in &self.failures {
            failure.append_recovery_records(&mut records);
        }
        records
    }
    /// 在后台回收之后调用；保留所有失败/未知记录，供上层报告或后续持久化。
    pub fn settle_shutdown(&mut self, remote: &mut Sessions, now_ms: u64) -> Result<()> {
        let close_error = self.close_all().err();
        self.poll(now_ms)?;
        while remote.deliver_surface(|outcome| self.accept_activation(outcome, now_ms, false)) {}
        if let Some(error) = close_error {
            return Err(error);
        }
        let unresolved = remote.surface_cleanup_pending()
            || self.failures.iter().any(|failure| !failure.settled())
            || self.entries.iter().any(|window| {
                window.is_alive()
                    || !window.javascript_finished()
                    || window.rejected_owner().is_some()
                    || !window.lifecycle().is_some_and(|owner| {
                        !owner.uncertain()
                            && matches!(
                                owner.binding().lifecycle(),
                                Lifecycle::Inactive | Lifecycle::Disposed
                            )
                    })
            });
        if unresolved {
            Err(Error::new(
                E_FAIL,
                "Surface shutdown has unresolved attachment state",
            ))
        } else {
            Ok(())
        }
    }
}
