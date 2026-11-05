//! 单字段异步草稿；权威值更新与请求完成分离，不把提交中的本地编辑提前覆盖。
use std::{collections::BTreeSet, rc::Rc};

pub const MAX_VALUE_BYTES: usize = 48 * 1024;
pub const MAX_PENDING: usize = 64;
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Value,
    Pending,
}
#[derive(Clone)]
pub struct Ticket {
    scope: Rc<()>,
    sequence: u64,
}
pub struct Draft {
    scope: Rc<()>,
    sequence: u64,
    source: String,
    value: String,
    pending: BTreeSet<u64>,
}
fn validate(value: &str) -> Result<(), Error> {
    if value.len() > MAX_VALUE_BYTES || value.contains('\0') {
        Err(Error::Value)
    } else {
        Ok(())
    }
}
impl Draft {
    pub fn new(source: &str) -> Result<Self, Error> {
        validate(source)?;
        Ok(Self {
            scope: Rc::new(()),
            sequence: 0,
            source: source.to_owned(),
            value: source.to_owned(),
            pending: BTreeSet::new(),
        })
    }
    pub fn value(&self) -> &str {
        &self.value
    }
    pub(crate) fn source(&self) -> &str {
        &self.source
    }
    pub fn pending(&self) -> usize {
        self.pending.len()
    }
    pub fn edit(&mut self, value: &str) -> Result<(), Error> {
        validate(value)?;
        self.value = value.to_owned();
        Ok(())
    }
    /// 同一作用域、未变化的权威字符串不能冲掉无异步 handler 的本地输入。
    pub fn synchronize(&mut self, source: &str) -> Result<(), Error> {
        validate(source)?;
        if source != self.source {
            self.source = source.to_owned();
            if self.pending.is_empty() {
                self.value.clone_from(&self.source);
            }
        }
        Ok(())
    }
    /// 异步派发前预留容量；派发未启动时 cancel，实际结果返回时 settle。不是动作权限。
    pub fn begin(&mut self) -> Result<Ticket, Error> {
        if self.pending.len() >= MAX_PENDING {
            return Err(Error::Pending);
        }
        let sequence = self.sequence.checked_add(1).ok_or(Error::Pending)?;
        self.pending.insert(sequence);
        self.sequence = sequence;
        Ok(Ticket {
            scope: self.scope.clone(),
            sequence,
        })
    }
    /// 旧 scope、外来或重复回执不生效；所有当前请求结算后才采用最新权威值。
    pub fn settle(&mut self, ticket: Ticket) -> bool {
        if !self.cancel(ticket) {
            return false;
        }
        if self.pending.is_empty() {
            self.value.clone_from(&self.source);
        }
        true
    }
    /// 未成功启动的任务释放预留，不把用户尚未提交的本地值回滚到旧权威值。
    pub fn cancel(&mut self, ticket: Ticket) -> bool {
        Rc::ptr_eq(&self.scope, &ticket.scope) && self.pending.remove(&ticket.sequence)
    }
    /// binding/node 身份变化时由上层明确重置；不能仅因 revision 变化就重置。
    pub fn reset(&mut self, source: &str) -> Result<(), Error> {
        let next = Self::new(source)?;
        *self = next;
        Ok(())
    }
}
#[cfg(test)]
#[path = "surface_input_draft_tests.rs"]
mod tests;
