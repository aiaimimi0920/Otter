//! 账号投射 UI 只持公开关联与显式请求；账号、网络和持久化由原本机 Loom/工作区拥有。
use super::*;
use crate::projection_account::{scene::Binding, Envelope};
use std::sync::{atomic::AtomicBool, Arc};

pub(in crate::desktop) enum Request {
    Create(share::SourceSelection),
    Import {
        envelope: Envelope,
        cancel: Arc<AtomicBool>,
    },
    Stop(Binding),
    Recover(crate::projection_account::journal::Intent),
}
#[derive(Default)]
pub(in crate::desktop) struct View {
    pub bindings: Vec<Binding>,
    pub busy: bool,
    pub status: String,
    pub received: Option<Envelope>,
}
