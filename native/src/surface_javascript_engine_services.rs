//! JS 栈退出后才进入私有宿主交换；只扣除该等待，不增加执行/布局/绘制预算。
use super::*;
impl Engine {
    pub(super) fn settle_services(
        &self,
        pending: Option<Persistent<Promise<'static>>>,
        dispatch: &mut dyn FnMut(wire::service::Call) -> io::Result<wire::service::ResultValue>,
    ) -> io::Result<()> {
        for _ in 0..=wire::service::MAX_CALLS {
            let finished = self
                .context
                .with(|ctx| match &pending {
                    Some(promise) => match promise.clone().restore(&ctx)?.finish::<()>() {
                        Ok(()) => Ok(true),
                        Err(rquickjs::Error::WouldBlock) => Ok(false),
                        Err(error) => Err(error),
                    },
                    None => Ok(true),
                })
                .map_err(|_| wire::invalid())?;
            self.drain_jobs()?;
            let next: String = self
                .context
                .with(|ctx| {
                    self.bridge
                        .clone()
                        .restore(&ctx)?
                        .get::<_, Function>("nextService")?
                        .call(())
                })
                .map_err(|_| wire::invalid())?;
            let call: Option<wire::service::Call> =
                serde_json::from_str(&next).map_err(|_| wire::invalid())?;
            let Some(mut call) = call else {
                return if finished {
                    Ok(())
                } else {
                    Err(wire::invalid())
                };
            };
            call.validate()?;
            // JS已在调用时flush选中Canvas；这里只编码该原生状态，不读取后续JS命令。
            if !call.canvases.is_empty() {
                self.canvas.borrow_mut().snapshots(&mut call.canvases)?;
                call.validate()?;
            }
            if !self.geometry.borrow().valid()
                || !self.canvas.borrow().valid()
                || self.origin.elapsed().as_micros()
                    >= u128::from(self.deadline.load(Ordering::Relaxed))
            {
                return Err(wire::invalid());
            }
            let id = call.id;
            let timeout = std::time::Duration::from_millis(u64::from(call.timeout_ms));
            // 没有 JS 在此时运行；等待由父侧绝对服务期限、取消和 Job owner 限制。
            let start = Instant::now();
            let image_output = call.image_output;
            let mut result = dispatch(call)?;
            let waited = start.elapsed();
            if waited >= timeout {
                return Err(wire::invalid());
            }
            self.deadline.fetch_add(
                waited.as_micros().min(u128::from(u64::MAX)) as u64,
                Ordering::Relaxed,
            );
            self.geometry.borrow_mut().exclude_wait(waited);
            self.canvas.borrow_mut().exclude_wait(waited);
            result.validate()?;
            if let wire::service::ResultValue::Image { image, png, .. } = &result {
                if !image_output {
                    return Err(wire::invalid());
                }
                let png = png.as_ref().ok_or_else(wire::invalid)?;
                if self.canvas.borrow_mut().append_image(image, png).is_err() {
                    // 本地安装事务失败不破坏旧资源/Canvas；只拒绝当前Promise。
                    result = wire::service::ResultValue::Failed {
                        error: "image_install_failed".into(),
                    };
                }
            } else if image_output && matches!(result, wire::service::ResultValue::Succeeded { .. })
            {
                return Err(wire::invalid());
            }
            let reply = wire::service::Reply {
                sequence: 0,
                id,
                result,
            };
            let json = serde_json::to_string(&reply).map_err(|_| wire::invalid())?;
            self.context
                .with(|ctx| {
                    self.bridge
                        .clone()
                        .restore(&ctx)?
                        .get::<_, Function>("settleService")?
                        .call::<_, ()>((json,))
                })
                .map_err(|_| wire::invalid())?;
        }
        Err(wire::invalid())
    }
}
