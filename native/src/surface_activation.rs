//! 激活事务的单 owner 移交；未确认期间禁止显示，失败不自动重发。
use crate::{
    surface_lifecycle_owner::Owner,
    surface_live_binding::{Binding, Lifecycle},
};
#[path = "surface_activation_batch.rs"]
mod batch;
pub struct Activation {
    binding: Binding,
    owner: Option<Owner>,
    attempted: bool,
    close_attempted: bool,
    pending: Option<Lifecycle>,
    pending_event: Option<String>,
    pending_batch: Option<Vec<String>>,
}
impl Activation {
    pub fn new(owner: Owner) -> Self {
        Self {
            binding: owner.binding().clone(),
            owner: Some(owner),
            attempted: false,
            close_attempted: false,
            pending: None,
            pending_event: None,
            pending_batch: None,
        }
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    pub fn owner(&self) -> Option<&Owner> {
        self.owner.as_ref()
    }
    /// 已确认活动附件的本地编辑资格，不等于可领取提交 owner；租约由窗口另验。
    pub fn local_edit_active(&self) -> bool {
        self.pending.is_none()
            && !self.close_attempted
            && (self.owner.as_ref().is_some_and(Owner::confirmed_active)
                || (self.owner.is_none()
                    && (self.pending_event.is_some() || self.pending_batch.is_some())))
    }
    pub fn into_owner(self) -> Option<Owner> {
        self.owner
    }
    /// 点击任务独占已激活 owner；不改变生命周期版本，不允许复用已尝试事件。
    pub fn take_event(&mut self, event: &crate::surface_click_event::ClickEvent) -> Option<Owner> {
        if event.attempted() {
            return None;
        }
        self.take_event_owner(event)
    }
    /// 决定任务复用原事件身份，不再提交原点击；租约与期限由窗口/HTTP 边界复核。
    pub fn take_decision(
        &mut self,
        event: &crate::surface_click_event::ClickEvent,
        ack: &crate::surface_action_ack::Ack,
        decision: &crate::surface_confirmation::Decision,
    ) -> Option<Owner> {
        if !event.attempted() || decision.attempted() || !decision.matches(event, ack) {
            return None;
        }
        self.take_event_owner(event)
    }
    fn take_event_owner(
        &mut self,
        event: &crate::surface_click_event::ClickEvent,
    ) -> Option<Owner> {
        let owner = self.owner.as_ref()?;
        if !owner.confirmed_active()
            || owner.binding() != event.binding()
            || self.pending.is_some()
            || self.pending_event.is_some()
            || self.pending_batch.is_some()
        {
            return None;
        }
        self.pending_event = Some(event.id().to_owned());
        self.owner.take()
    }
    /// 提交任务不改变生命周期；必须返回相同事件、绑定及确认状态的 owner。
    pub fn restore_event(
        &mut self,
        event: &crate::surface_click_event::ClickEvent,
        owner: Owner,
    ) -> Result<(), Box<Owner>> {
        if self.owner.is_some()
            || self.pending_batch.is_some()
            || self.pending.is_some()
            || self.pending_event.as_deref() != Some(event.id())
            || event.binding() != &self.binding
            || owner.binding() != &self.binding
            || !owner.confirmed_active()
        {
            return Err(Box::new(owner));
        }
        self.owner = Some(owner);
        self.pending_event = None;
        Ok(())
    }
    pub fn take(&mut self) -> Option<Owner> {
        let owner = self.owner.as_ref()?;
        if self.attempted
            || owner.uncertain()
            || owner.disposed()
            || owner.binding().lifecycle() != Lifecycle::Mounted
        {
            return None;
        }
        self.attempted = true;
        self.pending = Some(Lifecycle::Active);
        self.owner.take()
    }
    /// 关闭本地窗口只停用附件；不隐式销毁工作区节点或远端持久实例。
    pub fn take_inactive(&mut self) -> Option<Owner> {
        let owner = self.owner.as_ref()?;
        if self.close_attempted
            || owner.uncertain()
            || owner.disposed()
            || !matches!(
                owner.binding().lifecycle(),
                Lifecycle::Mounted | Lifecycle::Active
            )
        {
            return None;
        }
        self.close_attempted = true;
        self.pending = Some(Lifecycle::Inactive);
        self.owner.take()
    }
    /// 拒绝时归还传入 owner，不能丢失其他附件或迟到事务的恢复证据。
    pub fn restore(&mut self, owner: Owner) -> Result<(), Box<Owner>> {
        let old = &self.binding;
        let next = owner.binding();
        let identity = old.origin() == next.origin()
            && old.device() == next.device()
            && old.node() == next.node()
            && old.instance() == next.instance()
            && old.attachment() == next.attachment()
            && old.generation() == next.generation();
        // 未发出/unknown 可返回原确认状态；成功只能接受本次目标状态的下一版本。
        let version = old == next
            || (old.lifecycle_revision().checked_add(1) == Some(next.lifecycle_revision())
                && Some(next.lifecycle()) == self.pending);
        if self.pending.is_none() || self.owner.is_some() || !identity || !version {
            return Err(Box::new(owner));
        }
        self.binding = next.clone();
        self.owner = Some(owner);
        self.pending = None;
        Ok(())
    }
}
