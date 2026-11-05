//! 后台 Session 的私有服务交换。Dispatcher 是宿主接缝，不是授权；默认无派发权。
use super::*;
pub use wire::service::{Call, ResultValue};

/// 实现者必须在调用前后复核原授权，并让绝对期限/停止信号中断网络等待。
pub trait Dispatcher: Send {
    fn invoke(
        &mut self,
        call: Call,
        deadline: Instant,
        stopped: &dyn Fn() -> bool,
    ) -> io::Result<ResultValue>;
}
pub struct Mount {
    pub binding: Binding,
    pub source: String,
    pub snapshot: Value,
    pub resources: Value,
    pub images: Vec<ImageResource>,
    pub viewport: Option<crate::surface_viewport::Viewport>,
    pub dispatcher: Option<Box<dyn Dispatcher>>,
    pub(crate) allow_results: bool,
}
pub(super) struct Options {
    pub images: Vec<ImageResource>,
    pub viewport: Option<crate::surface_viewport::Viewport>,
    pub dispatcher: Option<Box<dyn Dispatcher>>,
    pub allow_results: bool,
}
impl Session {
    /// 仅供已授权后台 owner；此 API 不验签，不建立网络/设备凭据。
    pub fn mount_serviced(input: Mount, stopped: &impl Fn() -> bool) -> io::Result<(Self, Frame)> {
        Self::mount_options(
            input.binding,
            input.source,
            input.snapshot,
            input.resources,
            Options {
                images: input.images,
                viewport: input.viewport,
                dispatcher: input.dispatcher,
                allow_results: input.allow_results,
            },
            stopped,
            |endpoint, deadline| {
                worker::start(
                    endpoint,
                    deadline,
                    stopped,
                    worker::Policy {
                        suffix: if cfg!(test) {
                            " --exact surface_javascript_worker::tests::javascript_worker_entry --ignored --nocapture"
                        } else {
                            " --native-javascript-worker"
                        },
                        maximum: CHILD_LIMIT,
                        pool: &CAPACITY,
                        environment: worker::Environment::SystemRoot,
                    },
                )
            },
        )
    }
    pub(super) fn exchange_output(
        &mut self,
        command: Command,
        mut deadline: Instant,
        stopped: &impl Fn() -> bool,
    ) -> io::Result<(wire::Output, Instant)> {
        let child = self.child.as_ref().ok_or_else(wire::invalid)?;
        self.sequence = self.sequence.checked_add(1).ok_or_else(wire::invalid)?;
        wire::write(
            &mut self.pipe.cancellable_io(deadline, child.handle(), stopped),
            &wire::Request {
                sequence: self.sequence,
                viewport: self.viewport,
                command,
            },
        )?;
        let service_deadline = Instant::now() + Duration::from_secs(60);
        for id in 1..=wire::service::MAX_CALLS + 1 {
            let response: wire::Response =
                wire::read(&mut self.pipe.cancellable_io(deadline, child.handle(), stopped))?;
            if response.sequence != self.sequence || stopped() {
                return Err(wire::invalid());
            }
            let Some(call) = response.service else {
                #[cfg(test)]
                wire::diagnostics::log(response.diagnostic.as_ref());
                return response
                    .output
                    .map(|value| (value, deadline))
                    .ok_or_else(wire::invalid);
            };
            call.validate()?;
            if response.output.is_some() || call.id as usize != id || Instant::now() >= deadline {
                return Err(wire::invalid());
            }
            let due = service_deadline
                .min(Instant::now() + Duration::from_millis(u64::from(call.timeout_ms)));
            if Instant::now() >= due || stopped() {
                return Err(wire::invalid());
            }
            let dispatcher = self.dispatcher.as_mut().ok_or_else(wire::invalid)?;
            let start = Instant::now();
            let image_output = call.image_output;
            let result = dispatcher.invoke(call, due, stopped)?;
            let elapsed = start.elapsed();
            if stopped() || Instant::now() >= due {
                return Err(wire::invalid());
            }
            result.validate()?;
            if !image_output && matches!(result, ResultValue::Image { .. }) {
                return Err(wire::invalid());
            }
            // 仅排除同步 dispatcher 的等待，不重置 operation 的两秒传输/验证期限。
            deadline = deadline.checked_add(elapsed).ok_or_else(wire::invalid)?;
            let png = match &result {
                ResultValue::Image { png, .. } => Some(png.clone().ok_or_else(wire::invalid)?),
                _ => None,
            };
            wire::write(
                &mut self.pipe.cancellable_io(deadline, child.handle(), stopped),
                &wire::service::Reply {
                    sequence: self.sequence,
                    id: id as u32,
                    result,
                },
            )?;
            if let Some(png) = png {
                use std::io::Write;
                self.pipe
                    .cancellable_io(deadline, child.handle(), stopped)
                    .write_all(&png)?;
            }
        }
        Err(wire::invalid())
    }
}
