//! 自动回调复用签名Art串行槽；正式ACK只能在原Results事务成功之后产生。
use super::*;
use crate::art_live_trigger::{ticket::Ticket, Receipt, MAX_SEQUENCE};
impl Window {
    pub(in crate::desktop) fn accept_trigger(&mut self, ticket: Ticket) {
        if self.stopping || self.trigger.is_some() || !ticket.current(&self.current) {
            ticket.complete(Receipt::Cancelled);
            return;
        }
        self.trigger = Some(ticket);
        self.trigger_sent = false;
    }
    pub(super) fn advance_trigger(&mut self, host: &impl Fn() -> bool) -> Result<bool> {
        let Some(ticket) = self.trigger.clone() else {
            return Ok(false);
        };
        if !ticket.current(&self.current) {
            self.complete_trigger(Receipt::Cancelled);
            return Ok(false);
        }
        if self.trigger_sent {
            return Ok(true);
        }
        let Some(window) = &self.window else {
            return Ok(true);
        };
        // 已入FIFO的用户编辑优先，不重排或丢弃其已接受意图；timer不能饿死自动回调。
        if self.input.ticket.is_some()
            || self.input.deferred.is_some()
            || window.has_pending_edit()
            || window.has_pending_click()
        {
            return Ok(false);
        }
        let viewport = window.viewport()?;
        self.submit_input(
            crate::surface_javascript::signed::runtime::Operation::LiveTrigger(ticket, viewport),
            false,
            host,
        )?;
        self.trigger_sent = true;
        Ok(true)
    }
    pub(super) fn complete_trigger(&mut self, receipt: Receipt) {
        if let Some(ticket) = self.trigger.take() {
            ticket.complete(receipt);
        }
        self.trigger_sent = false;
    }
    pub(in crate::desktop) fn result_trigger(&self) -> Option<Ticket> {
        self.trigger_sent.then(|| self.trigger.clone()).flatten()
    }
    pub(in crate::desktop) fn can_commit_result(&self) -> bool {
        self.formal_revision < MAX_SEQUENCE
    }
    /// take_reply已清runtime的trigger槽；此处只核原执行授权，不重入自动gate锁或Win32。
    pub(in crate::desktop) fn result_authorized(&self) -> bool {
        !self.stopping && !self.runtime.pending() && self.runtime.validate(&self.current).is_ok()
    }
    pub(in crate::desktop) fn result_applied(&mut self) {
        self.formal_revision += 1;
        if self.trigger_sent {
            self.complete_trigger(Receipt::Applied {
                formal_revision: self.formal_revision,
            });
        }
    }
}
