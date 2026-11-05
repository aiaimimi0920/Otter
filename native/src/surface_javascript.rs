//! 显式原生 JS 子集会话；阻塞 API 仅供后台 owner/验证使用，默认 attach 不启用。
//! Job 限资源而非权限沙箱；父侧永不调用引擎，不接收脚本提供的绑定/版本。
use crate::{
    bounded_worker_process::{self as worker, Child},
    byte_budget::Budget,
    live_recovery_pipe::Pipe,
    surface_click_event::ClickEvent,
    surface_javascript_event as event,
    surface_javascript_wire::{self as wire, Command},
    surface_live_binding::Binding,
    surface_scene::Scene,
};
use serde_json::Value;
use std::{
    io::{self, Read},
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
#[path = "surface_javascript_authorized.rs"]
pub mod authorized;
#[path = "surface_javascript_mount.rs"]
mod mounting;
#[path = "surface_javascript_services.rs"]
pub mod services;
#[path = "surface_javascript_signed.rs"]
pub mod signed;
pub use wire::images::ImageResource;
#[cfg(test)]
#[path = "surface_javascript_image_session_tests.rs"]
mod image_session_tests;
pub(crate) const CHILD_LIMIT: usize = 128 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(2);
pub(crate) const MAX_SESSIONS: usize = 2;
pub(crate) const MAX_RESOURCES_BYTES: usize = 512 * 1024;
static CAPACITY: LazyLock<Arc<Budget>> = LazyLock::new(|| Budget::new(CHILD_LIMIT * MAX_SESSIONS));
/// 场景只用于呈现；事件已按当前权威 Scene 复核，发送仍须走原授权/FIFO/期限门。
pub struct Frame {
    pub(crate) result: Option<wire::result::Resolved>,
    pub scene: Scene,
    /// child 生成的当前 Canvas 像素；纯数据可跨后台线程，UI 才创建 target 资源。
    pub images: std::collections::BTreeMap<String, crate::image_decode::Image>,
    pub events: Vec<ClickEvent>,
    /// 每 operation 最多32项；只有父侧当前前台 Surface 内的 EDIT 可以接纳。
    pub(crate) focus: Vec<wire::focus::Request>,
    pub(crate) default_allowed: Option<bool>,
    /// 仅同步布局/DPR 读取时绑定 operation 开始的真实客户区；呈现前必须复核未变化。
    pub viewport: Option<crate::surface_viewport::Viewport>,
    /// 只表示 child 仍有 rAF；原 UI owner 决定授权、背压和实际下一帧期限。
    pub animation_pending: bool,
    /// child 单调时钟算出的剩余等待；宿主只用它安排原单槽，不能视为授权。
    pub timer_delay_ms: Option<u32>,
    /// 有观察目标才订阅真实宿主 resize；不是需要持续排帧的标记。
    pub resize_observed: bool,
}
pub struct Session {
    child: Option<Child>,
    pipe: Pipe,
    sequence: u64,
    binding: Binding,
    authority: Scene,
    revision: u64,
    suspended: bool,
    viewport: Option<crate::surface_viewport::Viewport>,
    dispatcher: Option<Box<dyn services::Dispatcher>>,
    allow_results: bool,
}
fn snapshot(value: &Value) -> io::Result<(Scene, u64)> {
    let bytes = wire::encode(value)?;
    if bytes.len() > crate::surface_scene::MAX_BYTES {
        return Err(wire::invalid());
    }
    let revision = value
        .get("revision")
        .and_then(Value::as_u64)
        .filter(|v| *v > 0)
        .ok_or_else(wire::invalid)?;
    let scene = wire::encode(value.get("scene").ok_or_else(wire::invalid)?)?;
    Ok((Scene::parse(&scene).map_err(|_| wire::invalid())?, revision))
}
pub(crate) fn resources(value: &Value) -> io::Result<()> {
    let object = value.as_object().ok_or_else(wire::invalid)?;
    if object.len() > 256
        || object
            .iter()
            .any(|(id, item)| !event::safe(id) || !item.is_string())
        || wire::encode(value)?.len() > MAX_RESOURCES_BYTES
    {
        return Err(wire::invalid());
    }
    Ok(())
}
impl Session {
    /// binding/snapshot 必须来自调用者已认证租约；这个 API 不自行赋予远端访问权。
    pub fn mount(
        binding: Binding,
        source: String,
        value: Value,
        assets: Value,
        stopped: &impl Fn() -> bool,
    ) -> io::Result<(Self, Frame)> {
        Self::mount_in(binding, source, value, assets, None, stopped)
    }
    /// 没有显式宿主视口时不猜测布局；普通 DOM 仍可使用 mount，连接节点几何读取会失败。
    pub fn mount_in(
        binding: Binding,
        source: String,
        value: Value,
        assets: Value,
        viewport: Option<crate::surface_viewport::Viewport>,
        stopped: &impl Fn() -> bool,
    ) -> io::Result<(Self, Frame)> {
        Self::mount_with_viewport(
            binding,
            source,
            value,
            assets,
            viewport,
            stopped,
            |endpoint, deadline| {
                let suffix = if cfg!(test) {
                    " --exact surface_javascript_worker::tests::javascript_worker_entry --ignored --nocapture"
                } else {
                    " --native-javascript-worker"
                };
                worker::start(
                    endpoint,
                    deadline,
                    stopped,
                    worker::Policy {
                        suffix,
                        maximum: CHILD_LIMIT,
                        pool: &CAPACITY,
                        environment: worker::Environment::SystemRoot,
                    },
                )
            },
        )
    }
    #[cfg(test)]
    fn mount_with(
        binding: Binding,
        source: String,
        value: Value,
        assets: Value,
        stopped: &impl Fn() -> bool,
        spawn: impl FnOnce(std::os::windows::io::OwnedHandle, Instant) -> io::Result<Child>,
    ) -> io::Result<(Self, Frame)> {
        Self::mount_with_viewport(binding, source, value, assets, None, stopped, spawn)
    }
    fn mount_with_viewport(
        binding: Binding,
        source: String,
        value: Value,
        assets: Value,
        viewport: Option<crate::surface_viewport::Viewport>,
        stopped: &impl Fn() -> bool,
        spawn: impl FnOnce(std::os::windows::io::OwnedHandle, Instant) -> io::Result<Child>,
    ) -> io::Result<(Self, Frame)> {
        Self::mount_options(
            binding,
            source,
            value,
            assets,
            services::Options {
                viewport,
                dispatcher: None,
                allow_results: false,
                images: Vec::new(),
            },
            stopped,
            spawn,
        )
    }
    pub fn set_viewport(&mut self, viewport: crate::surface_viewport::Viewport) -> io::Result<()> {
        viewport.validate().map_err(|_| wire::invalid())?;
        self.viewport = Some(viewport);
        Ok(())
    }
    pub fn update(
        &mut self,
        value: Value,
        assets: Value,
        stopped: &impl Fn() -> bool,
    ) -> io::Result<Frame> {
        let (authority, revision) = snapshot(&value)?;
        resources(&assets)?;
        if revision <= self.revision {
            return Err(wire::invalid());
        }
        // 新权威只在整个响应成功后留存；失败立即终止会话，禁止用旧协议流重试。
        let old = std::mem::replace(&mut self.authority, authority);
        let old_revision = std::mem::replace(&mut self.revision, revision);
        let result = self.exchange(
            Command::Update {
                snapshot: value,
                resources: assets,
            },
            Instant::now() + TIMEOUT,
            stopped,
        );
        if result.is_err() {
            self.authority = old;
            self.revision = old_revision;
        }
        result
    }
    pub fn click(&mut self, target: &str, stopped: &impl Fn() -> bool) -> io::Result<Frame> {
        if self.suspended || !event::safe(target) {
            return Err(wire::invalid());
        }
        self.exchange(
            Command::Click {
                target: target.into(),
            },
            Instant::now() + TIMEOUT,
            stopped,
        )
    }
    pub fn suspend(&mut self, stopped: &impl Fn() -> bool) -> io::Result<Frame> {
        let result = self.exchange(Command::Suspend, Instant::now() + TIMEOUT, stopped);
        if result.is_ok() {
            self.suspended = true;
        }
        result
    }
    pub(crate) fn live_trigger(
        &mut self,
        event: crate::art_live_trigger::Event,
        stopped: &impl Fn() -> bool,
    ) -> io::Result<Frame> {
        if !event.validate() {
            return Err(wire::invalid());
        }
        self.exchange(
            Command::LiveTrigger {
                event: Box::new(event),
            },
            Instant::now() + TIMEOUT,
            stopped,
        )
    }
    pub(crate) fn local_input(
        &mut self,
        input: impl Into<wire::LocalInput>,
        stopped: &impl Fn() -> bool,
    ) -> io::Result<Frame> {
        let input = input.into();
        input.validate()?;
        if self.suspended {
            return Err(wire::invalid());
        }
        self.exchange(
            Command::LocalInput { input },
            Instant::now() + TIMEOUT,
            stopped,
        )
    }
    pub fn animation_frame(&mut self, stopped: &impl Fn() -> bool) -> io::Result<Frame> {
        if self.suspended || self.viewport.is_none() {
            return Err(wire::invalid());
        }
        self.exchange(Command::AnimationFrame, Instant::now() + TIMEOUT, stopped)
    }
    pub fn resume(&mut self, stopped: &impl Fn() -> bool) -> io::Result<Frame> {
        let result = self.exchange(Command::Resume, Instant::now() + TIMEOUT, stopped);
        if result.is_ok() {
            self.suspended = false;
        }
        result
    }
    pub fn timer_tick(&mut self, stopped: &impl Fn() -> bool) -> io::Result<Frame> {
        if self.suspended {
            return Err(wire::invalid());
        }
        self.exchange(Command::TimerTick, Instant::now() + TIMEOUT, stopped)
    }
    pub fn dispose(mut self, stopped: &impl Fn() -> bool) -> io::Result<()> {
        let deadline = Instant::now() + TIMEOUT;
        self.exchange(Command::Dispose, deadline, stopped)?;
        self.child
            .as_ref()
            .ok_or_else(wire::invalid)?
            .finish(deadline, stopped)?;
        let mut extra = [0];
        match self.pipe.io(Some(deadline), None).read(&mut extra) {
            Ok(0) => {}
            Err(error) if closed(&error) => {}
            _ => return Err(wire::invalid()),
        }
        self.child.take();
        Ok(())
    }
    fn exchange(
        &mut self,
        command: Command,
        deadline: Instant,
        stopped: &impl Fn() -> bool,
    ) -> io::Result<Frame> {
        let result = self.exchange_inner(command, deadline, stopped);
        if result.is_err() {
            self.child.take();
        }
        result
    }
    fn exchange_inner(
        &mut self,
        command: Command,
        deadline: Instant,
        stopped: &impl Fn() -> bool,
    ) -> io::Result<Frame> {
        let disposing = matches!(command, Command::Dispose);
        let keyboard = matches!(&command, Command::LocalInput { input } if input.keyboard());
        let (output, deadline) = self.exchange_output(command, deadline, stopped)?;
        if let Some(result) = &output.result {
            result.validate()?;
            if !self.allow_results {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Local Art result owner unavailable",
                ));
            }
        }
        wire::focus::validate(&output.focus)?;
        if output.default_allowed.is_some() != keyboard
            || output.disposed != disposing
            || output.events.len() > 32
            || output
                .timer_delay_ms
                .is_some_and(|delay| delay > i32::MAX as u32)
            || (disposing
                && (!output.focus.is_empty()
                    || output.animation_pending
                    || output.resize_observed
                    || output.timer_delay_ms.is_some()))
        {
            return Err(wire::invalid());
        }
        let scene = wire::canvas::scene(&output.scene)?;
        let images = wire::canvas::images(&scene, output.canvases)?;
        let result = output
            .result
            .map(|request| request.resolve(&images))
            .transpose()?;
        let events = output
            .events
            .iter()
            .map(|value| event::encode(value, &self.authority, &self.binding, self.revision))
            .collect::<io::Result<_>>()?;
        if Instant::now() >= deadline || stopped() {
            return Err(wire::invalid());
        }
        let viewport = if output.layout_read {
            Some(self.viewport.ok_or_else(wire::invalid)?)
        } else {
            None
        };
        Ok(Frame {
            result,
            scene,
            images,
            events,
            focus: output.focus,
            default_allowed: output.default_allowed,
            viewport,
            animation_pending: output.animation_pending,
            timer_delay_ms: output.timer_delay_ms,
            resize_observed: output.resize_observed,
        })
    }
}
fn closed(error: &io::Error) -> bool {
    use windows::Win32::Foundation::{ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_NOT_CONNECTED};
    error.kind() == io::ErrorKind::BrokenPipe
        || error
            .get_ref()
            .and_then(|e| e.downcast_ref::<windows::core::Error>())
            .is_some_and(|e| {
                [ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_NOT_CONNECTED]
                    .iter()
                    .any(|code| e.code() == windows::core::HRESULT::from_win32(code.0))
            })
}
#[cfg(test)]
#[path = "surface_javascript_animation_tests.rs"]
pub(crate) mod animation_tests;
#[cfg(test)]
#[path = "surface_javascript_border_tests.rs"]
pub(crate) mod border_tests;
#[cfg(test)]
#[path = "surface_javascript_canvas_service_tests.rs"]
mod canvas_service_tests;
#[cfg(test)]
#[path = "surface_javascript_canvas_tests.rs"]
pub(crate) mod canvas_tests;
#[cfg(test)]
#[path = "surface_javascript_css_hover_tests.rs"]
pub(crate) mod css_hover_tests;
#[cfg(test)]
#[path = "surface_javascript_css_pressure_tests.rs"]
mod css_pressure_tests;
#[cfg(test)]
#[path = "surface_javascript_css_tests.rs"]
mod css_tests;
#[cfg(test)]
#[path = "surface_javascript_document_tests.rs"]
mod document_tests;
#[cfg(test)]
#[path = "surface_javascript_event_tests.rs"]
mod event_tests;
#[cfg(test)]
#[path = "surface_javascript_fixture.rs"]
pub(crate) mod fixture;
#[cfg(test)]
#[path = "surface_javascript_focus_tests.rs"]
pub(crate) mod focus_tests;
#[cfg(test)]
#[path = "surface_javascript_font_tests.rs"]
mod font_tests;
#[cfg(test)]
#[path = "surface_javascript_form_value_tests.rs"]
mod form_value_tests;
#[cfg(test)]
#[path = "surface_javascript_geometry_tests.rs"]
pub(crate) mod geometry_tests;
#[cfg(test)]
#[path = "surface_javascript_grid_tests.rs"]
mod grid_tests;
#[cfg(test)]
#[path = "surface_javascript_hover_tests.rs"]
pub(crate) mod hover_tests;
#[cfg(test)]
#[path = "surface_javascript_input_tests.rs"]
pub(crate) mod input_tests;
#[cfg(test)]
#[path = "surface_javascript_keyboard_tests.rs"]
pub(crate) mod keyboard_tests;
#[cfg(test)]
#[path = "surface_javascript_nth_tests.rs"]
pub(crate) mod nth_tests;
#[cfg(test)]
#[path = "surface_javascript_padding_tests.rs"]
mod padding_tests;
#[cfg(test)]
#[path = "surface_javascript_pointer_tests.rs"]
pub(crate) mod pointer_tests;
#[cfg(test)]
#[path = "surface_javascript_resize_tests.rs"]
pub(crate) mod resize_tests;
#[cfg(test)]
#[path = "surface_javascript_runtime_tests.rs"]
mod runtime_tests;
#[cfg(test)]
#[path = "surface_javascript_service_tests.rs"]
mod service_tests;
#[cfg(test)]
#[path = "surface_javascript_spatial_tests.rs"]
pub(crate) mod spatial_tests;
#[cfg(test)]
#[path = "surface_javascript_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "surface_javascript_text_overflow_tests.rs"]
pub(crate) mod text_overflow_tests;
#[cfg(test)]
#[path = "surface_javascript_timer_tests.rs"]
pub(crate) mod timer_tests;
#[cfg(test)]
#[path = "surface_javascript_typography_tests.rs"]
mod typography_tests;
