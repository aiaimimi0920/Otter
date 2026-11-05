//! 单附件生命周期事务；请求发出后的失败保留 unknown，禁止猜测服务端版本并自动重放。
use crate::surface_live_binding::{Binding, Error as BindingError, Lifecycle};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Uncertain,
    Disposed,
    State,
    Binding(BindingError),
}
pub struct Owner {
    binding: Binding,
    uncertain: bool,
    disposed: bool,
}
impl Owner {
    pub fn new(binding: Binding) -> Self {
        let disposed = binding.lifecycle() == Lifecycle::Disposed;
        Self {
            binding,
            uncertain: false,
            disposed,
        }
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    pub fn uncertain(&self) -> bool {
        self.uncertain
    }
    pub fn disposed(&self) -> bool {
        self.disposed
    }
    /// 只表示服务端生命周期已确认；显示仍必须通过当前设备授权检查。
    pub fn confirmed_active(&self) -> bool {
        !self.uncertain && !self.disposed && self.binding.lifecycle() == Lifecycle::Active
    }
    pub(crate) fn begin_recovery(&mut self) -> Result<(), Error> {
        if self.disposed {
            return Err(Error::Disposed);
        }
        self.uncertain = true;
        Ok(())
    }
    pub(crate) fn begin(&mut self, state: Lifecycle) -> Result<u64, Error> {
        if self.disposed {
            return Err(Error::Disposed);
        }
        if self.uncertain {
            return Err(Error::Uncertain);
        }
        if matches!(state, Lifecycle::Created | Lifecycle::Mounted) {
            return Err(Error::State);
        }
        let revision = self
            .binding
            .next_lifecycle_revision()
            .map_err(Error::Binding)?;
        self.uncertain = true;
        Ok(revision)
    }
    pub(crate) fn acknowledge(
        &mut self,
        bytes: &[u8],
        state: Lifecycle,
        revision: u64,
    ) -> Result<(), Error> {
        if !self.uncertain {
            return Err(Error::State);
        }
        let binding = self
            .binding
            .acknowledge(bytes, state, revision)
            .map_err(Error::Binding)?;
        self.binding = binding;
        self.uncertain = false;
        self.disposed = state == Lifecycle::Disposed;
        Ok(())
    }
    /// 只接受本次认证 remount 的绑定；旧缓存不能当作恢复证据。
    pub(crate) fn recover(&mut self, binding: Binding) -> Result<(), Error> {
        if self.disposed {
            return Err(Error::Disposed);
        }
        let old = &self.binding;
        if binding.generation() == old.generation()
            && binding.lifecycle_revision() < old.lifecycle_revision()
        {
            return Err(Error::Binding(BindingError::Revision));
        }
        if !binding.matches(old.origin(), old.device(), old.node(), binding.generation())
            || binding.instance() != old.instance()
            || binding.attachment() != old.attachment()
            || binding.generation() < old.generation()
        {
            return Err(Error::Binding(BindingError::Identity));
        }
        self.binding = binding;
        self.uncertain = false;
        Ok(())
    }
}
#[cfg(test)]
#[path = "surface_lifecycle_owner_tests.rs"]
mod tests;
