//! 执行请求所有者：每对象一个当前请求，代际匹配阻断过期交付；不持有线程或网络连接。
use crate::{workspace::MAX_UNITS, workspace_id::UnitId};
pub const MAX_PREVIEW_BYTES: usize = 65536;
pub const MAX_TOTAL_PREVIEW_BYTES: usize = 1024 * 1024;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub id: UnitId,
    pub generation: u64,
}
struct Entry {
    unit: UnitId,
    generation: u64,
    active: Option<Request>,
    preview: Option<String>,
}
#[derive(Default)]
pub struct Requests {
    entries: Vec<Entry>,
    preview_bytes: usize,
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Capacity,
    Allocation,
    GenerationExhausted,
    PreviewTooLarge,
}
impl Requests {
    /// 保留历史代际不算忙碌；任何节点的活动请求都阻止工作区整体替换。
    pub fn is_idle(&self) -> bool {
        self.entries.iter().all(|entry| entry.active.is_none())
    }
    pub(crate) fn next_generation(&self, unit: &UnitId) -> Result<u64, Error> {
        match self.entries.iter().find(|entry| &entry.unit == unit) {
            Some(entry) => entry
                .generation
                .checked_add(1)
                .ok_or(Error::GenerationExhausted),
            None if self.entries.len() >= MAX_UNITS => Err(Error::Capacity),
            None => Ok(1),
        }
    }
    /// 调用方提供新 request UUID；即使误复用 ID，仍要求交付方匹配完整代际令牌。
    pub fn begin(&mut self, unit: UnitId, id: UnitId) -> Result<Request, Error> {
        let index = match self.entries.iter().position(|entry| entry.unit == unit) {
            Some(index) => index,
            None => {
                if self.entries.len() >= MAX_UNITS {
                    return Err(Error::Capacity);
                }
                self.entries.try_reserve(1).map_err(|_| Error::Allocation)?;
                self.entries.push(Entry {
                    unit,
                    generation: 0,
                    active: None,
                    preview: None,
                });
                self.entries.len() - 1
            }
        };
        let entry = &mut self.entries[index];
        let generation = entry
            .generation
            .checked_add(1)
            .ok_or(Error::GenerationExhausted)?;
        let request = Request { id, generation };
        entry.generation = generation;
        entry.active = Some(request.clone());
        if let Some(preview) = entry.preview.take() {
            self.preview_bytes -= preview.len();
        }
        Ok(request)
    }
    pub fn active(&self, unit: &UnitId) -> Option<&Request> {
        self.entries
            .iter()
            .find(|entry| &entry.unit == unit)?
            .active
            .as_ref()
    }
    pub fn is_latest(&self, unit: &UnitId, request: &Request) -> bool {
        self.active(unit) == Some(request)
    }
    /// 已由协议解码器限制消息字节；这里接管字符串所有权，不额外复制图片引用。
    pub fn mark_preview(
        &mut self,
        unit: &UnitId,
        request: &Request,
        preview: String,
    ) -> Result<bool, Error> {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| &entry.unit == unit && entry.active.as_ref() == Some(request))
        else {
            return Ok(false);
        };
        let old_bytes = entry.preview.as_ref().map_or(0, String::len);
        let next_bytes = self.preview_bytes - old_bytes + preview.len();
        if preview.len() > MAX_PREVIEW_BYTES || next_bytes > MAX_TOTAL_PREVIEW_BYTES {
            return Err(Error::PreviewTooLarge);
        }
        entry.preview = Some(preview);
        self.preview_bytes = next_bytes;
        Ok(true)
    }
    pub fn preview(&self, unit: &UnitId, request: &Request) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| &entry.unit == unit && entry.active.as_ref() == Some(request))?
            .preview
            .as_deref()
    }
    pub fn finish(&mut self, unit: &UnitId, request: &Request) -> bool {
        if !self.is_latest(unit, request) {
            return false;
        }
        self.invalidate(unit);
        true
    }
    /// 取消保留 generation；下一次 begin 单调增加，旧回调不能重新成为当前请求。
    pub fn invalidate(&mut self, unit: &UnitId) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| &entry.unit == unit) {
            entry.active = None;
            if let Some(preview) = entry.preview.take() {
                self.preview_bytes -= preview.len();
            }
        }
    }
    /// 对象关闭时移除元数据并释放预算；同一工作区的新对象必须使用新 UnitId。
    pub fn retain_units(&mut self, mut keep: impl FnMut(&UnitId) -> bool) {
        self.entries.retain(|entry| keep(&entry.unit));
        self.preview_bytes = self
            .entries
            .iter()
            .filter_map(|entry| entry.preview.as_ref())
            .map(String::len)
            .sum();
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.preview_bytes = 0;
    }
    pub fn preview_bytes(&self) -> usize {
        self.preview_bytes
    }
}
#[cfg(test)]
#[path = "graph_requests_tests.rs"]
mod tests;
