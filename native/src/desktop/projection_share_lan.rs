//! LAN invitations reuse send bindings and receive journals; links contain only signed invitations, never session credentials.
use super::*;
use crate::projection_delivery::{sender::binding::Binding, share};
use crate::ui_text::text as tr;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
impl Model {
    pub(super) fn lan_binding(&self) -> Option<Binding> {
        let view = self.view.upgrade()?;
        let view = view.borrow();
        view.sender
            .bindings
            .iter()
            .find(|b| {
                b.unit() == self.unit.as_str()
                    && b.envelope().origin() == self.origin
                    && b.is_invitation()
            })
            .cloned()
    }
    unsafe fn queue_lan(&self, request: Request) -> Result<()> {
        let view = self
            .view
            .upgrade()
            .ok_or_else(|| Error::new(E_ABORT, "Share closed"))?;
        let mut view = view.borrow_mut();
        if view.closed || super::busy(&view) || view.origin.as_deref() != Some(&self.origin) {
            return Err(Error::new(E_ABORT, tr("share-busy")));
        }
        view.request = Some(request);
        if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
            view.request = None;
            return Err(error);
        }
        Ok(())
    }
    pub(super) unsafe fn start_lan(&self) -> Result<()> {
        self.queue_lan(Request::ShareProjection(SourceSelection {
            unit: self.unit.clone(),
            origin: self.origin.clone(),
            target: self.target.clone(),
            cancel: self.cancel.clone(),
        }))
    }
    pub(super) unsafe fn stop_lan(&self) -> Result<()> {
        let Some(binding) = self.lan_binding() else {
            return self.start_lan();
        };
        // A pending unlink still owns the canonical association.
        if binding.stopped() {
            return Err(Error::new(E_ABORT, tr("share-unavailable")));
        }
        let view = self
            .view
            .upgrade()
            .ok_or_else(|| Error::new(E_ABORT, "Share closed"))?;
        let source = view
            .borrow()
            .sender
            .stop_source
            .clone()
            .ok_or_else(|| Error::new(E_ABORT, tr("share-source-unavailable")))?;
        self.queue_lan(Request::StopProjectionSend(Box::new(
            super::super::sender::StopSelection { binding, source },
        )))
    }
    pub(super) fn lan_output_current(&self) -> bool {
        !self.receive
            && !self.cancel.load(Ordering::Acquire)
            && self.hwnd.get().is_some()
            && self.view.upgrade().is_some_and(|v| {
                let v = v.borrow();
                !v.closed && v.origin.as_deref() == Some(&self.origin)
            })
            && self.lan_binding().is_some_and(|b| {
                !b.stopped()
                    && b.envelope().expires_at_ms()
                        > crate::desktop::surface_windows::now_ms().unwrap_or(u64::MAX)
                    && self.envelope.borrow().as_ref() == Some(b.envelope())
            })
    }
    pub(super) unsafe fn parse_lan(&self, text: &str) -> bool {
        self.envelope.borrow_mut().take();
        let Ok(link) = share::parse(text) else {
            return false;
        };
        *self.source_loom_id.borrow_mut() = link.source_loom_id;
        *self.envelope.borrow_mut() = Some(link.envelope);
        self.set_status("");
        if let Some(hwnd) = self.hwnd.get() {
            window::enable(hwnd, 1, true);
        }
        true
    }
    pub(super) unsafe fn submit_lan(&self) -> Result<()> {
        let envelope = self
            .envelope
            .borrow()
            .clone()
            .ok_or_else(|| Error::new(E_INVALIDARG, tr("share-lan-invalid")))?;
        self.queue_lan(Request::Projection(
            super::super::projection::Request::Import(Arc::new(share::Import {
                envelope,
                source_loom_id: self.source_loom_id.borrow().clone(),
                cancel: self.cancel.clone(),
            })),
        ))?;
        self.account_import.set(true);
        self.set_status(tr("share-importing"));
        Ok(())
    }
    pub(super) unsafe fn refresh_lan(&self) {
        let (Some(hwnd), Some(view)) = (self.hwnd.get(), self.view.upgrade()) else {
            return;
        };
        let (invalid, busy, status, received) = {
            let v = view.borrow();
            (
                v.closed || v.origin.as_deref() != Some(&self.origin),
                v.projection.busy || matches!(v.request, Some(Request::Projection(_))),
                if self.receive {
                    v.projection.status.clone()
                } else {
                    v.sender.status.clone()
                },
                self.envelope.borrow().as_ref().is_some_and(|e| {
                    v.projection
                        .recovery
                        .saved
                        .iter()
                        .any(|t| t.envelope() == e)
                }),
            )
        };
        if invalid || (self.receive && self.account_import.get() && received) {
            let _ = DestroyWindow(hwnd);
            return;
        }
        let binding = (!self.receive).then(|| self.lan_binding()).flatten();
        let (label, enabled) = match binding.as_ref() {
            Some(binding) if binding.stopped() => ("share-unavailable", false),
            Some(_) => ("share-unlink", true),
            None => ("share-generate", true),
        };
        window::enable(hwnd, 108, enabled && !self.account_import.get());
        window::label(self, 108, label);
        if self.receive {
            if self.account_import.get() {
                self.set_status(&status);
                if !busy {
                    self.account_import.set(false);
                }
            }
            window::enable(
                hwnd,
                1,
                !self.account_import.get() && self.envelope.borrow().is_some(),
            );
        } else if let Some(binding) = binding {
            if binding.stopped()
                || binding.envelope().expires_at_ms()
                    <= crate::desktop::surface_windows::now_ms().unwrap_or(u64::MAX)
            {
                self.set_status(tr("share-invitation-expired"));
                window::output(hwnd, self, false);
            } else if self.envelope.borrow().as_ref() != Some(binding.envelope())
                || self.link.borrow().is_empty()
            {
                let Some(loom_id) = view.borrow().loom_id.clone() else {
                    self.set_status(tr("share-loom-not-ready"));
                    return;
                };
                match share::encode(binding.envelope(), &loom_id)
                    .map_err(|_| tr("share-encode-failed").to_owned())
                    .and_then(|link| {
                        *self.code.borrow_mut() = Some(qr::encode_bytes(link.as_bytes())?);
                        *self.link.borrow_mut() = link;
                        *self.envelope.borrow_mut() = Some(binding.envelope().clone());
                        Ok(())
                    }) {
                    Ok(()) => {
                        self.set_status("");
                        window::output(hwnd, self, true);
                    }
                    Err(error) => self.set_status(&error),
                }
            }
        } else {
            self.set_status(if status.is_empty() {
                tr("share-generating")
            } else {
                &status
            });
            window::output(hwnd, self, false);
        }
        for id in [101, 103, 104, 105, 109, 110] {
            window::enable(hwnd, id, !self.account_import.get());
        }
        window::place(self);
    }
    pub(super) unsafe fn switch_scope(&self, public: bool) -> Result<()> {
        if self.public.get() == public {
            return Ok(());
        }
        if self.account_import.get() {
            return Err(Error::new(E_ABORT, tr("share-importing")));
        }
        self.public.set(public);
        self.envelope.borrow_mut().take();
        self.account_envelope.borrow_mut().take();
        self.code.borrow_mut().take();
        self.link.borrow_mut().clear();
        self.cancel_decode();
        self.set_status("");
        // Both scopes use the same window size, but the receiver's unbind button is visible only for account associations.
        if let Some(hwnd) = self.hwnd.get() {
            if !self.receive {
                window::input(hwnd, "");
            }
            window::enable(hwnd, 1, false);
            layout::arrange(hwnd, self, GetDpiForWindow(hwnd));
            for id in [109, 110] {
                if let Ok(button) = GetDlgItem(Some(hwnd), id) {
                    let _ =
                        windows::Win32::Graphics::Gdi::InvalidateRect(Some(button), None, false);
                }
            }
        }
        if self.receive {
            let input = self.inputs.borrow()[usize::from(public)].clone();
            self.parsed(&input);
        } else {
            self.create_pending.set(true);
        }
        Ok(())
    }
}
