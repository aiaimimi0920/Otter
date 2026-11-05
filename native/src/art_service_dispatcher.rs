//! 原 Prepared 向 signed Local 移交服务上下文；共享同一 Permit/Lease，不复制凭据或准入时钟。
use super::*;
use crate::{
    art_execution_grant::Permit, device_authorization_lease::Lease, surface_javascript::services,
};
use std::{
    io,
    sync::{Arc, Mutex},
};
#[path = "art_service_failure.rs"]
mod failures;

struct Dispatcher {
    permit: Arc<Mutex<Permit>>,
    lease: Arc<Lease>,
    binding: grant_wire::Binding,
    envelope: Vec<u8>,
    client: Client,
    progress: Option<Arc<progress::Channel>>,
    images: Vec<crate::surface_javascript::ImageResource>,
}
pub(crate) fn prepare(
    prepared: &Prepared,
    online: bool,
    progress: Option<Arc<progress::Channel>>,
    images: &[crate::surface_javascript::ImageResource],
) -> Result<Option<Box<dyn services::Dispatcher>>, super::super::Error> {
    if !prepared
        .request
        .permissions
        .contains(&grant_wire::Permission::LoomService)
    {
        return Ok(None);
    }
    let unsupported =
        || super::super::Error::Mount(crate::surface_javascript::signed::Error::Unsupported);
    if !online && prepared.lease.is_local() {
        return Ok(Some(Box::new(Offline {
            services: prepared.request.services.clone(),
        })));
    }
    if !online {
        return Err(unsupported());
    }
    let client = prepared.client.clone().ok_or_else(unsupported)?;
    Ok(Some(Box::new(Dispatcher {
        permit: prepared.permit.clone(),
        lease: prepared.lease.clone(),
        binding: prepared.request.binding.clone(),
        envelope: prepared.envelope.clone(),
        client,
        progress,
        images: images.to_vec(),
    })))
}
struct Offline {
    services: Vec<String>,
}
impl services::Dispatcher for Offline {
    fn invoke(
        &mut self,
        request: services::Call,
        deadline: Instant,
        stopped: &dyn Fn() -> bool,
    ) -> io::Result<services::ResultValue> {
        request.validate()?;
        if stopped() || Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "art service stopped",
            ));
        }
        if !self.services.contains(&request.service) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "art service scope",
            ));
        }
        Ok(services::ResultValue::Failed {
            error: "offline".into(),
        })
    }
}
fn failure(error: Error) -> io::Error {
    // 不将凭据、原始 HTTP 或签名内容透传给不可信脚本；完整执行失败由宿主 owner 处理。
    io::Error::other(format!("art service exchange failed: {error:?}"))
}
impl services::Dispatcher for Dispatcher {
    fn invoke(
        &mut self,
        request: services::Call,
        deadline: Instant,
        stopped: &dyn Fn() -> bool,
    ) -> io::Result<services::ResultValue> {
        request.validate()?;
        let check = || {
            if stopped() || self.lease.execution_stopped() || Instant::now() >= deadline {
                Err(Error::Stopped)
            } else {
                Ok(())
            }
        };
        check().map_err(failure)?;
        let mut call = Call::begin(
            Authority {
                permit: &self.permit,
                lease: &self.lease,
                binding: &self.binding,
                envelope: &self.envelope,
            },
            &self.client,
            &request.service,
            request.input,
            u64::from(request.timeout_ms),
        )
        .map_err(failure)?;
        call.image_output = request.image_output;
        if request.confirmation {
            call.request_confirmation().map_err(failure)?;
        }
        if !request.images.is_empty() {
            call.attach_images(&request.images, &self.images)
                .map_err(failure)?;
        } else if !request.canvases.is_empty() {
            call.attach_canvases(request.canvases).map_err(failure)?;
        }
        // 父 operation 的服务总期限不能被每次调用重新开始；原许可还可进一步缩短。
        call.deadline = call.deadline.min(deadline);
        call.timeout_ms = call.timeout_ms.min(
            call.deadline
                .saturating_duration_since(Instant::now())
                .as_millis() as u64,
        );
        if call.timeout_ms == 0 {
            return Err(failure(Error::Time));
        }
        let result = (|| {
            let mut reply = call.submit()?;
            loop {
                check()?;
                if call.challenge.is_some() {
                    reply = match Self::confirm(&self.progress, &self.lease, &mut call, &check)? {
                        Some(approved) => call.decide(approved)?,
                        None => call.prior.clone().ok_or(Error::Sequence)?,
                    };
                    continue;
                }
                // Reply已由Call复核身份、原授权与序号；只交付有界宿主状态，不发布Frame。
                if let Some(progress) = &self.progress {
                    progress.publish(&reply);
                }
                match reply.status {
                    Status::Succeeded => {
                        if request.image_output {
                            let image = call.output_image.take().ok_or(Error::Wire)?;
                            // 消费后沿原owner取消端点释放Loom终态图片；不重新执行，也不延长许可。
                            call.cancel()?;
                            let mut retained = self.images.clone();
                            retained.push(image.clone());
                            if crate::surface_javascript_wire::images::headers(&retained).is_err() {
                                return Ok(services::ResultValue::Failed {
                                    error: "image_output_capacity".into(),
                                });
                            }
                            self.images = retained;
                            let mut headers = crate::surface_javascript_wire::images::headers(
                                std::slice::from_ref(&image),
                            )
                            .map_err(|_| Error::Wire)?;
                            return Ok(services::ResultValue::Image {
                                output: reply.output.ok_or(Error::Wire)?,
                                image: headers.remove(0),
                                png: Some(image.png),
                            });
                        }
                        return Ok(services::ResultValue::Succeeded {
                            output: reply.output.ok_or(Error::Wire)?,
                        });
                    }
                    Status::Failed => {
                        return Ok(services::ResultValue::Failed {
                            error: reply.error.ok_or(Error::Wire)?,
                        })
                    }
                    Status::Cancelled => {
                        return Ok(services::ResultValue::Failed {
                            error: "cancelled".into(),
                        })
                    }
                    Status::Accepted | Status::Running => {}
                }
                // 有界串行 poll；执行停止中断网络，等待最多20ms后复核，不新增线程。
                let remaining = call.deadline.saturating_duration_since(Instant::now());
                self.lease.wait(remaining.min(Duration::from_millis(20)));
                check()?;
                reply = call.poll()?;
            }
        })();
        match result {
            Ok(value) => Ok(value),
            Err(error) => self.settle_failure(call, error, deadline, stopped),
        }
    }
}
impl Dispatcher {
    fn confirm(
        progress: &Option<Arc<progress::Channel>>,
        lease: &Lease,
        call: &mut Call<'_>,
        check: &impl Fn() -> Result<(), Error>,
    ) -> Result<Option<bool>, Error> {
        let Some(channel) = progress else {
            return Ok(Some(false));
        };
        let images = confirmation::preview::prepare(call, &|| {
            check().is_err() || Instant::now() >= call.deadline
        })?;
        let mut text = call.confirmation_text()?;
        for image in &images {
            text.push_str(&format!(
                "\r\n{}：{} × {}",
                image.label, image.size[0], image.size[1]
            ));
        }
        let ticket = channel
            .confirmations
            .publish_images(text, call.deadline, images)?;
        let mut next_poll = Instant::now() + Duration::from_millis(100);
        let result = (|| loop {
            check()?;
            call.authorize(false, false)?;
            if let Some(approved) = ticket.choice() {
                return Ok(Some(approved));
            }
            if !ticket.live() {
                return Err(Error::Stopped);
            }
            // 等待用户期间仍观察同一个Task，远端撤权/换代不留下可批准的陈旧提示。
            if Instant::now() >= next_poll {
                if call.poll()?.status.terminal() {
                    return Ok(None);
                }
                if call.challenge.is_none() {
                    return Err(Error::Wire);
                }
                next_poll = Instant::now() + Duration::from_millis(100);
            }
            lease.wait(
                call.deadline
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(20)),
            );
        })();
        channel.confirmations.retire(&ticket);
        result
    }
}
#[cfg(test)]
#[path = "art_service_offline_tests.rs"]
mod offline_tests;
