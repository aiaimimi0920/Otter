//! 接收 owner 的终止与稳定化：普通取消不 join，实际磁盘包含事实同步撤销自动补偿资格。
use super::*;
use crate::projection_delivery::{binding::checkpoint::Index, journal::Ticket};

impl Saved {
    pub(super) fn stabilize(&mut self) {
        self.held.stabilize();
        self.durable = true;
    }
}
impl Owner {
    pub(in crate::desktop::application) fn stop(&mut self, cleanup: &mut super::cleanup::Owner) {
        self.1 = Default::default();
        match &self.0 {
            Phase::Preparing(pending) => pending.job.stop(), // UI 不 join，仍独占原槽/活动许可。
            Phase::Reporting(reporting) => reporting.job.request_stop(),
            Phase::Recovering(pending) => pending.job.request_stop(),
            Phase::Syncing(pending) => pending.job.request_stop(),
            _ => {
                let phase = std::mem::take(&mut self.0);
                let ticket = match &phase {
                    Phase::Checkpoint(saved) => saved.held.cleanup_ticket(),
                    _ => None,
                };
                drop(phase);
                if let Some(ticket) = ticket {
                    cleanup.abandon(ticket);
                }
            }
        }
    }
    pub(in crate::desktop::application) fn shutdown(
        &mut self,
        cleanup: &mut super::cleanup::Owner,
    ) {
        self.stop(cleanup);
        let phase = std::mem::take(&mut self.0);
        let ticket = match &phase {
            Phase::Preparing(pending) => pending.job.cleanup_ticket(),
            _ => None, // Reporting 必须已实际保存；回执失败不是删除证据。
        };
        drop(phase); // 只在终态同步回收，先结束原 worker/活动许可，再排 marker。
        if let Some(ticket) = ticket {
            cleanup.abandon(ticket);
        }
    }
    pub(in crate::desktop::application) fn committed(
        &mut self,
        index: Option<&Index>,
        workspace: &crate::workspace::Workspace,
    ) {
        if let Phase::Checkpoint(saved) = &mut self.0 {
            if saved.workspace.matches(workspace) {
                match index {
                    Some(index) if index.includes(&saved.expected) => saved.stabilize(),
                    // 旧/坏关联不阻止普通保存；索引未知不能证明未保存，更不能自动解绑。
                    // 只降级到显式恢复，不授 durable、呈现或 displayed 资格。
                    None => saved.held.stabilize(),
                    _ => {}
                }
            }
        }
    }
}
impl Application {
    pub(super) fn receive_cleanup(&mut self, ticket: Option<Ticket>) {
        if let Some(ticket) = ticket {
            self.projection_cleanup.abandon(ticket);
        }
    }
}
