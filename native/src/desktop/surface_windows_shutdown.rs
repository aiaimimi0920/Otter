//! Surface 终态门禁：正常回收与退出检查共用同一判定，不重试、不冒充持久化。
use super::*;
use crate::{
    desktop::composition::relay::sessions::Sessions,
    surface_lifecycle_owner::Owner,
    surface_live_binding::Lifecycle,
    surface_recovery_record::{self, Record},
};

fn terminal_owner(window: &AuthorizedWindow) -> Option<&Owner> {
    if window.is_alive() || !window.javascript_finished() || window.rejected_owner().is_some() {
        return None;
    }
    window.lifecycle().filter(|owner| {
        !owner.uncertain()
            && matches!(
                owner.binding().lifecycle(),
                Lifecycle::Inactive | Lifecycle::Disposed
            )
    })
}

impl Windows {
    pub(crate) fn retired_records(&self) -> &[Record] {
        &self.retired
    }

    /// 先有可落盘的提示，再释放 owner；超预算不淘汰既有证据，也不在消息泵写盘。
    /// 返回 true 只报告首次阻止退休，避免每轮重复通知。
    pub(crate) fn retire_confirmed(&mut self) -> bool {
        if self.retirement_blocked {
            return false;
        }
        let mut index = 0;
        while let Some(window) = self.entries.get(index) {
            let Some(owner) = terminal_owner(window) else {
                index += 1;
                continue;
            };
            let record =
                Record::from_binding(owner.binding(), Some(window.request().art()), false, false);
            if !self.stage_retirement(record) {
                return true;
            }
            self.entries.remove(index);
        }
        let mut index = 0;
        while let Some(failure) = self.failures.get(index) {
            let Some(owner) = failure.terminal_owner() else {
                index += 1;
                continue;
            };
            let record = Record::from_binding(
                owner.binding(),
                Some(failure.outcome.request.art()),
                false,
                false,
            );
            if !self.stage_retirement(record) {
                return true;
            }
            self.failures.remove(index);
        }
        false
    }

    fn stage_retirement(&mut self, record: Record) -> bool {
        if self.retired.len() < surface_recovery_record::MAX_RECORDS
            && self.retired.try_reserve(1).is_ok()
        {
            self.retired.push(record);
            if surface_recovery_record::encode(&self.retired).is_ok() {
                return true;
            }
            self.retired.pop();
        }
        self.retirement_blocked = true;
        false
    }

    pub fn recovery_records(&self) -> Vec<Record> {
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
            || self
                .entries
                .iter()
                .any(|window| terminal_owner(window).is_none());
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
