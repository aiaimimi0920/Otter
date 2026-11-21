//! 分享只使用账号 v2；二维码解析不联网，只有创建/导入/解绑按钮排队业务。
use super::*;
use crate::desktop::device_authorization_ui::account::Request as AccountRequest;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
impl Model {
    pub(super) unsafe fn ensure_account(&self) -> Result<()> {
        let active = self.view.upgrade().is_some_and(|v| {
            v.borrow().account.bindings.iter().any(|b| {
                b.unit() == self.unit.as_str() && b.origin() == self.origin && !b.stopped()
            })
        });
        if active {
            Ok(())
        } else {
            self.stop_account()
        }
    }
    unsafe fn queue_account(&self, request: AccountRequest) -> Result<()> {
        let view = self
            .view
            .upgrade()
            .ok_or_else(|| Error::new(E_ABORT, "Share owner closed"))?;
        let mut view = view.borrow_mut();
        if view.closed || super::busy(&view) || view.origin.as_deref() != Some(&self.origin) {
            return Err(Error::new(E_ABORT, "账号投射正在处理，请稍后重试"));
        }
        view.request = Some(Request::AccountProjection(Box::new(request)));
        if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
            view.request = None;
            return Err(error);
        }
        Ok(())
    }
    pub(super) unsafe fn parse_account(&self, text: &str) -> bool {
        let Ok(envelope) = crate::projection_account::Envelope::parse(text) else {
            return false;
        };
        self.set_status("");
        *self.account_envelope.borrow_mut() = Some(envelope);
        if let Some(hwnd) = self.hwnd.get() {
            window::enable(hwnd, 1, true);
        }
        true
    }
    pub(super) unsafe fn submit_account(&self) -> Result<()> {
        let envelope = self
            .account_envelope
            .borrow()
            .clone()
            .ok_or_else(|| Error::new(E_INVALIDARG, "无效的账号邀请"))?;
        self.queue_account(AccountRequest::Import {
            envelope,
            cancel: self.cancel.clone(),
        })?;
        self.account_import.set(true);
        self.set_status("正在导入…");
        Ok(())
    }
    pub(super) unsafe fn stop_account(&self) -> Result<()> {
        let view = self
            .view
            .upgrade()
            .ok_or_else(|| Error::new(E_ABORT, "Share owner closed"))?;
        let binding = view
            .borrow()
            .account
            .bindings
            .iter()
            .find(|b| b.unit() == self.unit.as_str() && b.origin() == self.origin && !b.stopped())
            .cloned();
        if let Some(binding) = binding {
            self.queue_account(AccountRequest::Stop(binding))
        } else if !self.receive {
            self.queue_account(AccountRequest::Create(SourceSelection {
                unit: self.unit.clone(),
                origin: self.origin.clone(),
                target: self.target.clone(),
                cancel: self.cancel.clone(),
            }))
        } else {
            Err(Error::new(E_ABORT, "没有活动的账号关联"))
        }
    }
    pub(super) fn account_output_current(&self) -> bool {
        if self.receive || self.cancel.load(Ordering::Acquire) || self.hwnd.get().is_none() {
            return false;
        }
        let Some(view) = self.view.upgrade() else {
            return false;
        };
        let view = view.borrow();
        let now = crate::desktop::surface_windows::now_ms().unwrap_or(u64::MAX);
        !view.closed
            && view.origin.as_deref() == Some(&self.origin)
            && view.account.bindings.iter().any(|b| {
                b.unit() == self.unit.as_str()
                    && b.origin() == self.origin
                    && b.is_source()
                    && !b.stopped()
                    && b.envelope().expires_at_ms() > now
                    && self.account_envelope.borrow().as_ref() == Some(b.envelope())
            })
    }
    pub(super) unsafe fn refresh_account(&self) {
        let Some(hwnd) = self.hwnd.get() else {
            return;
        };
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let (invalid, busy, status, binding, received) = {
            let v = view.borrow();
            (
                v.closed || v.origin.as_deref() != Some(&self.origin),
                v.account.busy || matches!(v.request, Some(Request::AccountProjection(_))),
                v.account.status.clone(),
                v.account
                    .bindings
                    .iter()
                    .find(|b| b.unit() == self.unit.as_str() && b.origin() == self.origin)
                    .cloned(),
                self.account_import.get()
                    && v.account.received.as_ref() == self.account_envelope.borrow().as_ref(),
            )
        };
        if invalid || received {
            let _ = DestroyWindow(hwnd);
            return;
        }
        let active = binding.as_ref().is_some_and(|b| !b.stopped());
        if self.receive && self.account_import.get() {
            self.set_status(&status);
            if !busy {
                self.account_import.set(false);
            }
        }
        if self.receive {
            if let Ok(button) = GetDlgItem(Some(hwnd), 108) {
                if IsWindowVisible(button).as_bool() != active {
                    layout::arrange(hwnd, self, GetDpiForWindow(hwnd));
                }
            }
        }
        window::label(
            hwnd,
            108,
            if active {
                "解除关联"
            } else {
                "生成邀请"
            },
        );
        window::enable(
            hwnd,
            108,
            !self.account_import.get() && (active || !self.receive),
        );
        for id in [104, 105, 109, 110] {
            window::enable(hwnd, id, !self.account_import.get());
        }
        if self.receive {
            window::enable(
                hwnd,
                1,
                !self.account_import.get() && self.account_envelope.borrow().is_some(),
            );
            for id in [101, 103, 104, 105] {
                window::enable(hwnd, id, !self.account_import.get());
            }
        } else if let Some(binding) = binding.as_ref().filter(|b| b.is_source()) {
            if binding.stopped()
                || binding.envelope().expires_at_ms()
                    <= crate::desktop::surface_windows::now_ms().unwrap_or(u64::MAX)
            {
                self.set_status(if binding.stopped() {
                    "投射已停止"
                } else {
                    "邀请已到期，已接收关联继续同步"
                });
                window::output(hwnd, self, false);
            } else if self.account_envelope.borrow().as_ref() != Some(binding.envelope()) {
                let result = (|| {
                    let envelope = binding.envelope();
                    let link = envelope.link().map_err(|_| "邀请编码失败".to_owned())?;
                    *self.link.borrow_mut() = link.clone();
                    *self.code.borrow_mut() = Some(qr::encode_bytes(link.as_bytes())?);
                    *self.account_envelope.borrow_mut() = Some(envelope.clone());
                    Ok::<_, String>(())
                })();
                match result {
                    Ok(()) => {
                        self.set_status("");
                        window::output(hwnd, self, true);
                    }
                    Err(error) => self.set_status(&error),
                }
            }
        } else if binding.is_some() {
            self.set_status(if active {
                "正在接收账号投射"
            } else {
                "关联已解除，保留最后图像"
            });
            window::output(hwnd, self, false);
        } else {
            self.set_status(if status.is_empty() {
                "正在生成账号邀请…"
            } else {
                &status
            });
            window::output(hwnd, self, false);
        }
        window::place(self);
    }
}
