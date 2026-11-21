//! 单任务的 UI rendezvous；后台取得许可后才请求资源，只有一个在途请求，无 HWND/State 跨线程。
use super::{extension_command_route::Context, image::Image, loom_execution_signal::Notify};
use crate::{
    loom_bridge_stream::Interrupt,
    loom_extension_session::{Error, Outcome},
    unit_attachment_store::Store,
};
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, SyncSender},
    Arc, Mutex,
};
pub(super) enum Request {
    Input,
    Toggle { translation: bool },
    Resources { image: bool, attachments: bool },
    Apply { command: String, outcome: Outcome },
}
pub(super) enum Reply {
    Input(Option<Value>),
    Toggled(bool),
    Resources {
        image: Option<Image>,
        store: Option<Store>,
    },
    Applied(Context),
}
pub(super) struct Packet {
    pub(super) request: Request,
    reply: SyncSender<Result<Reply, Error>>,
}
impl Packet {
    pub(super) fn answer(self, result: Result<Reply, Error>) {
        let _ = self.reply.send(result);
    }
}
#[derive(Default)]
pub(super) struct Mailbox {
    pub(super) packet: Option<Packet>,
    pub(super) interrupt: Option<Interrupt>,
}
#[derive(Clone)]
pub(super) struct Port {
    pub(super) mailbox: Arc<Mutex<Mailbox>>,
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) notify: Notify,
}
impl Port {
    pub(super) fn check(&self) -> Result<(), Error> {
        if self.cancel.load(Ordering::Acquire) {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
    pub(super) fn ask(&self, request: Request) -> Result<Reply, Error> {
        let (tx, rx) = mpsc::sync_channel(1);
        {
            let mut mailbox = self.mailbox.lock().unwrap_or_else(|e| e.into_inner());
            self.check()?;
            assert!(mailbox.packet.is_none(), "extension rendezvous overlapped");
            mailbox.packet = Some(Packet { request, reply: tx });
        }
        (self.notify)();
        // UI 模态/停止调度不能把关闭 join 变成无限等待；取消会丢弃唯一 sender，立即唤醒。
        let result = rx.recv_timeout(std::time::Duration::from_secs(5));
        self.check()?;
        result.map_err(|e| match e {
            mpsc::RecvTimeoutError::Timeout => Error::Timeout,
            mpsc::RecvTimeoutError::Disconnected => Error::Closed,
        })?
    }
    pub(super) fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
        let mut mailbox = self.mailbox.lock().unwrap_or_else(|e| e.into_inner());
        mailbox.packet.take();
        if let Some(interrupt) = &mailbox.interrupt {
            let _ = interrupt.cancel();
        }
    }
}
