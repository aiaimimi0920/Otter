//! 后台顺序派送：每条连接建立后重新准入，成功才推进；不自动重试或回滚远端。
use super::{exchange, Batch, Request};
use crate::{byte_budget::Budget, loom_bridge_client, loom_bridge_identity::Identity};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Debug, PartialEq)]
pub enum Failure {
    Prepare(super::Error),
    Connect(loom_bridge_client::Error),
    NotAdmitted,
    Cancelled,
    Exchange(exchange::Error),
}
#[derive(Debug, PartialEq)]
pub struct Report {
    /// 仅本次调用收到严格匹配成功回执的数量，不包含之前调用已确认的消息。
    pub acknowledged: usize,
    pub failure: Option<Failure>,
    /// 当前消息是否可能到达远端；false 也不能否定此前尝试已经提交。
    pub may_have_reached_remote: bool,
}
/// 必须在后台执行。admit 不可直接访问 HWND/Rc 状态，应由 owner 提供线程安全
/// 门禁或与 UI 握手；返回 true 只授权当前请求，不给整批永久授权。
/// 失败保留 batch 的当前请求及 ID；调用方不得未经重新核查就再次调用来自动重试。
pub fn run(
    batch: &mut Batch<'_>,
    identity: &Identity,
    budget: &Arc<Budget>,
    limit: usize,
    cancel: &AtomicBool,
    mut admit: impl FnMut(&Request) -> bool,
) -> Report {
    let mut report = Report {
        acknowledged: 0,
        failure: None,
        may_have_reached_remote: false,
    };
    loop {
        if cancel.load(Ordering::Acquire) {
            report.failure = Some(Failure::Cancelled);
            return report;
        }
        let request = match batch.current(budget, limit, cancel) {
            Ok(Some(request)) => request,
            Ok(None) => return report,
            Err(error) => {
                report.failure = Some(Failure::Prepare(error));
                return report;
            }
        };
        let socket = match loom_bridge_client::connect(identity, Duration::from_secs(5)) {
            Ok(socket) => socket,
            Err(error) => {
                report.failure = Some(Failure::Connect(error));
                return report;
            }
        };
        // 连接可能耗时；只有连接完成后的新门禁可以授权消息内容的发送。
        if cancel.load(Ordering::Acquire) {
            report.failure = Some(Failure::Cancelled);
            return report;
        }
        if !admit(request) {
            report.failure = Some(Failure::NotAdmitted);
            return report;
        }
        match exchange::run(socket, request, budget, cancel) {
            Ok(()) => {
                let id = request.id().to_owned();
                // 独占 batch 借用，当前 request 不可能在交换期间被另一个 owner 推进。
                assert!(batch.acknowledge(&id));
                report.acknowledged += 1;
            }
            Err(error) => {
                // Cancelled 可能在发送前，也可能在等待回执时；不能当成明确未提交。
                report.may_have_reached_remote = !matches!(error, exchange::Error::Budget);
                report.failure = Some(Failure::Exchange(error));
                return report;
            }
        }
    }
}
