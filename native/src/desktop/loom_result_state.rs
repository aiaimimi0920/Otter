//! 单节点结果 owner：预览不传播到正式端口，最终提交保留本轮预览并原子合并端口。
use super::{
    image::Image,
    loom_output_batch::{Batch, Output, Port},
};
use crate::{graph_requests::Request, workspace_id::UnitId};
use std::sync::Arc;
#[path = "art_local_result_state.rs"]
mod local_result;
#[path = "loom_result_restore_values.rs"]
mod restore_values;
#[path = "loom_result_save_values.rs"]
mod save_values;

/// 不透明正式输出凭证；不持有像素，也不会因计数溢出或重建同名节点而复用。
#[derive(Clone)]
pub struct OutputStamp(Arc<()>);
impl OutputStamp {
    pub fn matches(&self, results: &Results) -> bool {
        Arc::ptr_eq(&self.0, &results.output_stamp.0)
    }
}
pub struct ImageSource {
    pub image: super::loom_image_value::ImageValue,
    pub stamp: OutputStamp,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Cancelled,
    InvalidOutput,
    Stale,
    Capacity,
    Allocation,
    Budget,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Idle,
    Running,
    Completed,
    Failed,
    Cancelled,
}
pub struct Results {
    surface: Option<surface::Version>,
    surface_revoked: bool,
    output_stamp: OutputStamp,
    status: Status,
    progress: Option<f64>,
    error: Option<String>,
    unit: UnitId,
    active: Option<Request>,
    preview_revision: u64,
    workflow_preview: Option<Image>,
    display: Option<Image>,
    restored_preview_locked: bool,
    ports: Vec<Port>,
    formal_outputs_committed: bool,
    candidates: Option<serde_json::Value>,
    candidate_view: Option<Arc<super::loom_output_batch::candidates::Set>>,
    candidate_configuration: Option<super::configuration_stamp::Stamp>,
}
impl Results {
    pub fn new(unit: UnitId) -> Self {
        Self {
            surface: None,
            surface_revoked: false,
            output_stamp: OutputStamp(Arc::new(())),
            status: Status::Idle,
            progress: None,
            error: None,
            unit,
            active: None,
            preview_revision: 0,
            workflow_preview: None,
            display: None,
            restored_preview_locked: false,
            ports: Vec::new(),
            formal_outputs_committed: false,
            candidates: None,
            candidate_view: None,
            candidate_configuration: None,
        }
    }
    pub fn unit(&self) -> &UnitId {
        &self.unit
    }
    /// 会话图像仅作为初始显示，不伪造已完成请求、正式端口或 workflow preview。
    pub fn from_restored_display(unit: UnitId, display: Option<Image>, locked: bool) -> Self {
        Self {
            restored_preview_locked: locked && display.is_some(),
            display,
            ..Self::new(unit)
        }
    }
    pub fn holds_restored_preview(&self) -> bool {
        self.restored_preview_locked
    }
    /// 只解除保持标记；不清空旧画面或伪造正式输出。执行开关与该标记互相独立。
    pub fn invalidate_restored_preview(&mut self) {
        self.restored_preview_locked = false;
    }
    pub fn begin(&mut self, request: Request) {
        self.surface = None;
        self.surface_revoked = true;
        self.invalidate_restored_preview();
        self.status = Status::Running;
        self.progress = None;
        self.error = None;
        self.active = Some(request);
        self.preview_revision = 0;
        self.workflow_preview = None;
        self.clear_candidates();
        // 与主干一致：启动新执行不清掉已有正式输出/画面。
    }
    pub fn display(&self) -> Option<&Image> {
        self.display.as_ref()
    }
    pub fn ports(&self) -> &[Port] {
        &self.ports
    }
    pub fn output_stamp(&self) -> OutputStamp {
        self.output_stamp.clone()
    }
    /// 只取正式端口；预览和 display 不参与执行来源选择。图像像素共享，不复制。
    pub fn image_source(&self, name: &str) -> Option<ImageSource> {
        let port = self.ports.iter().find(|port| port.name == name)?;
        Some(ImageSource {
            image: port.value.image_value()?,
            stamp: self.output_stamp(),
        })
    }
    pub fn candidates(&self) -> Option<&serde_json::Value> {
        self.candidates.as_ref()
    }
    pub(crate) fn receive_candidates(&mut self, candidates: Option<serde_json::Value>) {
        self.candidates = candidates;
    }
    pub(crate) fn bind_candidate_configuration(
        &mut self,
        stamp: super::configuration_stamp::Stamp,
    ) {
        self.candidate_configuration = self.candidate_view.as_ref().map(|_| stamp);
    }
    pub(crate) fn candidate_view(
        &self,
        stamp: &super::configuration_stamp::Stamp,
    ) -> Option<&Arc<super::loom_output_batch::candidates::Set>> {
        (self.status == Status::Completed && self.candidate_configuration.as_ref() == Some(stamp))
            .then_some(self.candidate_view.as_ref())
            .flatten()
    }
    fn clear_candidates(&mut self) {
        self.candidates = None;
        self.candidate_view = None;
        self.candidate_configuration = None;
    }
    pub fn active(&self) -> Option<&Request> {
        self.active.as_ref()
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn progress(&self) -> Option<f64> {
        self.progress
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    /// 只接收当前请求的有限进度值，不猜测远端使用百分比还是归一化单位。
    pub fn set_progress(&mut self, request: &Request, progress: f64) -> bool {
        if self.active.as_ref() != Some(request) || !progress.is_finite() {
            return false;
        }
        self.progress = Some(progress);
        true
    }
    /// 用户可见错误有界，失败保留原显示和正式端口；不写日志或复制协议原始响应。
    pub fn fail(&mut self, request: &Request, message: &str) -> bool {
        if self.active.as_ref() != Some(request) {
            return false;
        }
        self.error = Some(message.chars().take(1024).collect());
        self.status = Status::Failed;
        self.active = None;
        self.workflow_preview = None;
        true
    }
    pub fn preview(
        &mut self,
        request: &Request,
        revision: u64,
        batch: Batch,
    ) -> Result<bool, Error> {
        if self.active.as_ref() != Some(request) {
            return Err(Error::Stale);
        }
        if revision == 0 || revision <= self.preview_revision {
            return Ok(false);
        }
        self.preview_revision = revision;
        let Some(image) = batch.primary().value.image() else {
            return Ok(false);
        };
        self.workflow_preview = Some(image.clone());
        self.display = Some(image.clone());
        Ok(true)
    }
    /// 调用方先确认文档/图配置新鲜度，再在同一 UI 调度段提交；远端结果须先释放租约。
    /// 本地结果无需也不得伪造远端 lease/delivery；该层不替代应用的来源/配置核验。
    /// 失败不修改旧结果；输入已是纯本地 owner，失败可安全回收。
    pub fn finish(&mut self, request: &Request, batch: Batch) -> Result<(), Error> {
        if self.active.as_ref() != Some(request) {
            return Err(Error::Stale);
        }
        let image = batch.primary().value.image_value();
        let candidates = batch.candidates.clone();
        let (mut incoming, _) = batch.into_ports();
        // 主干图像最终输出同时覆盖 output 和 output_image，像素仅共享，不复制。
        if let Some(image) = &image {
            incoming.try_reserve(2).map_err(|_| Error::Allocation)?;
            for name in ["output", "output_image"] {
                if let Some(port) = incoming.iter_mut().find(|port| port.name == name) {
                    port.value = Output::from_image(image.clone());
                } else {
                    incoming.push(Port {
                        name: name.into(),
                        value: Output::from_image(image.clone()),
                    });
                }
            }
        }
        let retained = self
            .ports
            .iter()
            .filter(|old| !incoming.iter().any(|new| new.name == old.name));
        let count = incoming.len() + retained.clone().count();
        if count > 128 {
            return Err(Error::Capacity);
        }
        let mut bytes = Meter(0);
        for port in retained.chain(incoming.iter()) {
            match &port.value {
                Output::Image(image) => {
                    bytes.add(image.pixels.len()).map_err(|_| Error::Budget)?;
                }
                Output::PngImage(image) => {
                    bytes.add(image.bytes()).map_err(|_| Error::Budget)?;
                }
                Output::Value(value) => {
                    serde_json::to_writer(&mut bytes, value).map_err(|_| Error::Budget)?
                }
            }
        }
        // 所有失败点先处理，再移动旧端口；正式输出总预算采用保守的别名重复计数。
        let mut merged = Vec::new();
        merged.try_reserve(count).map_err(|_| Error::Allocation)?;
        let output_stamp = OutputStamp(Arc::new(()));
        for old in self.ports.drain(..) {
            if !incoming.iter().any(|new| new.name == old.name) {
                merged.push(old);
            }
        }
        merged.append(&mut incoming);
        self.ports = merged;
        self.formal_outputs_committed = true;
        self.output_stamp = output_stamp;
        self.clear_candidates();
        self.candidate_view = candidates;
        self.display = self
            .workflow_preview
            .take()
            .or_else(|| image.map(|value| value.image().clone()))
            .or_else(|| self.display.take());
        self.active = None;
        self.status = Status::Completed;
        self.progress = Some(1.0);
        self.error = None;
        Ok(())
    }
    pub fn cancel(&mut self, request: &Request) -> bool {
        if self.active.as_ref() != Some(request) {
            return false;
        }
        self.active = None;
        self.workflow_preview = None;
        self.status = Status::Cancelled;
        self.error = None;
        true
    }
}
#[path = "surface_node_results.rs"]
mod surface;
// 避免为了预算检查先序列化一个同等大小的 JSON 字符串。
struct Meter(usize);
impl Meter {
    fn add(&mut self, count: usize) -> std::io::Result<()> {
        self.0 = self
            .0
            .checked_add(count)
            .filter(|value| *value <= 256 * 1024 * 1024)
            .ok_or_else(|| std::io::Error::other("result budget exceeded"))?;
        Ok(())
    }
}
impl std::io::Write for Meter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.add(bytes.len())?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "loom_result_state_tests.rs"]
mod tests;
