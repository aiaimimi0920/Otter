//! 新执行许可保护的本地预览接缝；复用真实 JS child，不复用旧 Surface 的资源 lease 作为许可。
//! 阻塞 API 仅供后台 owner。当前只接本地状态/预览；服务派发、正式结果与 UI 接线仍由上层负责。
//! 宿主生命周期失效必须进入 cancelled；grant Binding 不含窗口/附件生命周期，不可替代其 owner。
use super::{Frame, Session};
use crate::{
    art_execution_grant::{self as grant, wire, Expected, Issuer, Permit},
    device_authorization_lease::Lease,
    surface_live_binding::Binding,
};
use std::{
    cell::RefCell,
    io,
    sync::{Arc, Mutex},
};
#[cfg(test)]
#[path = "surface_javascript_signed_result_tests.rs"]
mod result_tests;
#[path = "surface_javascript_signed_runtime.rs"]
pub mod runtime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Grant(grant::Error),
    Lease,
    Binding,
    Unsupported,
    Stopped,
    Execution(io::ErrorKind),
}
/// 两种身份分别来自当前宿主请求；不得从签名响应反向补造 Expected。
pub struct Input<'a> {
    pub envelope: &'a [u8],
    pub issuer: Issuer<'a>,
    pub expected: Expected<'a>,
    pub source: &'a [u8],
    pub view: Binding,
    pub snapshot: serde_json::Value,
    pub resources: serde_json::Value,
    pub images: Vec<super::ImageResource>,
    pub viewport: Option<crate::surface_viewport::Viewport>,
}
pub struct Local {
    permit: Arc<Mutex<Permit>>,
    lease: Arc<Lease>,
    session: Option<Session>,
}
pub(crate) enum Interaction {
    Input(crate::surface_javascript_wire::LocalInput),
    Click(String),
}
fn now() -> Result<u64, Error> {
    super::authorized::now().map_err(|_| Error::Stopped)
}
fn check(
    permit: &Mutex<Permit>,
    lease: &Lease,
    current: &wire::Binding,
    online: bool,
    cancelled: &impl Fn() -> bool,
) -> Result<(), Error> {
    let result = (|| {
        let time = now()?;
        if cancelled() {
            return Err(Error::Stopped);
        }
        lease
            .validate(&current.origin, time)
            .map_err(|_| Error::Lease)?;
        if lease.execution_stopped() {
            return Err(Error::Stopped);
        }
        if lease.device_id() != current.device_id {
            return Err(Error::Binding);
        }
        permit
            .lock()
            .map_err(|_| Error::Stopped)?
            .authorize(current, time, online, lease.is_revoked())
            .map_err(Error::Grant)?;
        Ok(())
    })();
    if result.is_err() {
        permit.lock().unwrap_or_else(|e| e.into_inner()).stop();
    }
    result
}
impl Local {
    /// 不建立 issuer 信任、不下载源码、不创建另一套 JS 引擎；先验签再分配 child。
    pub fn mount(
        input: Input<'_>,
        lease: Arc<Lease>,
        online: bool,
        cancelled: &impl Fn() -> bool,
    ) -> Result<(Self, Frame), Error> {
        Self::mount_inner(input, lease, online, cancelled, None, None)
    }
    /// 已下载结果保留首次准入时钟；仍重新验证输入字节，不能用 Permit 替代源码完整性。
    pub(crate) fn mount_prepared(
        input: Input<'_>,
        lease: Arc<Lease>,
        online: bool,
        cancelled: &impl Fn() -> bool,
        permit: Arc<Mutex<Permit>>,
        dispatcher: Option<Box<dyn super::services::Dispatcher>>,
    ) -> Result<(Self, Frame), Error> {
        Self::mount_inner(
            input,
            lease,
            online,
            cancelled,
            Some((permit, false)),
            dispatcher,
        )
    }
    pub(crate) fn mount_resumed(
        input: Input<'_>,
        lease: Arc<Lease>,
        cancelled: &impl Fn() -> bool,
        permit: Arc<Mutex<Permit>>,
        dispatcher: Option<Box<dyn super::services::Dispatcher>>,
    ) -> Result<(Self, Frame), Error> {
        if !lease.is_local() {
            return Err(Error::Lease);
        }
        Self::mount_inner(
            input,
            lease,
            false,
            cancelled,
            Some((permit, true)),
            dispatcher,
        )
    }
    fn mount_inner(
        input: Input<'_>,
        lease: Arc<Lease>,
        online: bool,
        cancelled: &impl Fn() -> bool,
        prior: Option<(Arc<Mutex<Permit>>, bool)>,
        dispatcher: Option<Box<dyn super::services::Dispatcher>>,
    ) -> Result<(Self, Frame), Error> {
        let current = input.expected.binding;
        if input.view.origin() != current.origin
            || input.view.device() != current.device_id
            || input.view.instance() != current.instance_id
            || input.view.generation() != current.generation
            || input.view.lifecycle() != crate::surface_live_binding::Lifecycle::Active
        {
            return Err(Error::Binding);
        }
        let resumed = prior.as_ref().is_some_and(|(_, resumed)| *resumed);
        let accept = if resumed {
            Permit::accept_resumed
        } else {
            Permit::accept
        };
        let accepted = accept(
            input.envelope,
            input.issuer,
            input.expected,
            input.source,
            now()?,
        )
        .map_err(Error::Grant)?;
        let permit = prior
            .map(|(permit, _)| permit)
            .unwrap_or_else(|| Arc::new(Mutex::new(accepted)));
        // 仅已下载 owner 能注入原许可 dispatcher；旧 emit 仍不是服务或正式结果。
        let scope = permit
            .lock()
            .map_err(|_| Error::Stopped)?
            .authorize(current, now()?, online, false)
            .map_err(Error::Grant)?
            .permissions
            .clone();
        if !scope.contains(&wire::Permission::LocalState)
            || !scope.contains(&wire::Permission::Preview)
            || (scope.contains(&wire::Permission::LoomService)
                && (dispatcher.is_none() || (!online && !resumed)))
        {
            return Err(Error::Unsupported);
        }
        check(&permit, &lease, current, online, cancelled)?;
        let failure = RefCell::new(None);
        let stopped = || match check(&permit, &lease, current, online, cancelled) {
            Ok(()) => false,
            Err(error) => {
                *failure.borrow_mut() = Some(error);
                true
            }
        };
        let source = std::str::from_utf8(input.source)
            .map_err(|_| Error::Binding)?
            .to_owned();
        let result = Session::mount_serviced(
            super::services::Mount {
                binding: input.view,
                source,
                snapshot: input.snapshot,
                resources: input.resources,
                images: input.images,
                viewport: input.viewport,
                dispatcher,
                allow_results: scope.contains(&wire::Permission::WorkspaceResult),
            },
            &stopped,
        );
        let (session, frame) =
            result.map_err(|e| failure.into_inner().unwrap_or(Error::Execution(e.kind())))?;
        let mut local = Self {
            permit,
            lease,
            session: Some(session),
        };
        local.refresh(current, online, cancelled)?;
        local.frame(frame).map(|frame| (local, frame))
    }
    /// 闲置 owner 必须在授权截止/撤权唤醒时调用；本接缝不另起计时线程。
    pub fn refresh(
        &mut self,
        current: &wire::Binding,
        online: bool,
        cancelled: &impl Fn() -> bool,
    ) -> Result<(), Error> {
        if self.session.is_none() {
            return Err(Error::Stopped);
        }
        if let Err(error) = check(&self.permit, &self.lease, current, online, cancelled) {
            self.stop();
            return Err(error);
        }
        Ok(())
    }
    fn frame(&mut self, frame: Frame) -> Result<Frame, Error> {
        if !frame.events.is_empty() {
            self.stop();
            return Err(Error::Unsupported);
        }
        Ok(frame)
    }
    fn operate(
        &mut self,
        current: &wire::Binding,
        online: bool,
        cancelled: &impl Fn() -> bool,
        execute: impl FnOnce(&mut Session, &dyn Fn() -> bool) -> io::Result<Frame>,
    ) -> Result<Frame, Error> {
        self.refresh(current, online, cancelled)?;
        let failure = RefCell::new(None);
        let result = execute(
            self.session.as_mut().ok_or(Error::Stopped)?,
            &|| match check(&self.permit, &self.lease, current, online, cancelled) {
                Ok(()) => false,
                Err(error) => {
                    *failure.borrow_mut() = Some(error);
                    true
                }
            },
        );
        let frame = match result {
            Ok(frame) => frame,
            Err(error) => {
                self.stop();
                return Err(failure
                    .into_inner()
                    .unwrap_or(Error::Execution(error.kind())));
            }
        };
        self.refresh(current, online, cancelled)?;
        self.frame(frame)
    }
    pub fn click(
        &mut self,
        current: &wire::Binding,
        online: bool,
        target: &str,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Frame, Error> {
        self.operate(current, online, cancelled, |session, stop| {
            session.click(target, &stop)
        })
    }
    pub fn suspend(
        &mut self,
        current: &wire::Binding,
        online: bool,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Frame, Error> {
        self.operate(current, online, cancelled, |session, stop| {
            session.suspend(&stop)
        })
    }
    pub fn resume(
        &mut self,
        current: &wire::Binding,
        online: bool,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Frame, Error> {
        self.operate(current, online, cancelled, |session, stop| {
            session.resume(&stop)
        })
    }
    pub(crate) fn scheduled(
        &mut self,
        current: &wire::Binding,
        online: bool,
        viewport: crate::surface_viewport::Viewport,
        animation: Option<bool>,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Frame, Error> {
        self.operate(current, online, cancelled, |session, stop| {
            session.set_viewport(viewport)?;
            match animation {
                Some(true) => session.animation_frame(&stop),
                Some(false) => session.timer_tick(&stop),
                None => session.resume(&stop),
            }
        })
    }
    pub(crate) fn input(
        &mut self,
        current: &wire::Binding,
        online: bool,
        input: Interaction,
        viewport: crate::surface_viewport::Viewport,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Frame, Error> {
        self.operate(current, online, cancelled, |session, stop| {
            session.set_viewport(viewport)?;
            match input {
                Interaction::Input(input) => session.local_input(input, &stop),
                Interaction::Click(target) => session.click(&target, &stop),
            }
        })
    }
    /// 自动回调只沿原串行child执行；每个阻塞接缝同时核本次领取及签名授权。
    pub(crate) fn live_trigger(
        &mut self,
        current: &wire::Binding,
        online: bool,
        ticket: crate::art_live_trigger::ticket::Ticket,
        viewport: crate::surface_viewport::Viewport,
    ) -> Result<Frame, Error> {
        self.operate(
            current,
            online,
            &|| !ticket.current(current),
            |session, stop| {
                session.set_viewport(viewport)?;
                session.live_trigger(ticket.event().clone(), &stop)
            },
        )
    }
    /// 不执行不可信 dispose 回调；只回收自有 child，不撤销其他 owner 的设备租约。
    pub fn stop(&mut self) {
        self.permit.lock().unwrap_or_else(|e| e.into_inner()).stop();
        self.session.take();
    }
}
#[cfg(test)]
#[path = "surface_javascript_signed_tests.rs"]
mod tests;
