//! Single-slot callback for local modal buttons: separate submission from completion, and collect callbacks within a short cancellation budget.
use super::*;
#[path = "live_input_native_request.rs"]
pub mod native_request;
use std::{
    cell::Cell,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
#[path = "live_input_native_motion.rs"]
pub mod native_motion;
pub use native_motion::motion_native;

thread_local! {
    // The callback carries only a value token, never Backend/Controller/stack/heap pointers.
    static CALLBACK: Cell<(usize, bool)> = const { Cell::new((0, false)) };
}
static NEXT: AtomicUsize = AtomicUsize::new(1);

#[derive(Default)]
pub struct Modal {
    token: usize,
    escape_edges: u8,
    destination: Option<Destination>,
    owner: Option<crate::live_hover_owner::Control>,
    popup_scope: otter_live_hover_hook::native_input_scope::Snapshot,
    publication: Option<native_request::Publication>,
    rejected: bool,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
unsafe extern "system" fn completed(_window: HWND, _message: u32, token: usize, _result: LRESULT) {
    CALLBACK.with(|slot| {
        if slot.get().0 == token {
            slot.set((token, true));
        }
    });
}
impl Modal {
    pub fn set_popup_scope(&mut self, scope: otter_live_hover_hook::native_input_scope::Snapshot) {
        self.popup_scope = scope;
    }
    pub fn set_owner(&mut self, owner: crate::live_hover_owner::Control) {
        self.owner = Some(owner);
    }
    pub fn check_result(&mut self) -> Result<()> {
        self.refresh();
        if self.rejected {
            Err(Error::DeliveryFailed)
        } else {
            Ok(())
        }
    }
    /// Determine slot emptiness only after check_result succeeds; do not pump callbacks again here, or a newly arrived
    /// rejection would clear the token while bypassing the completed error check and admitting the next input.
    fn require_idle(&self) -> Result<()> {
        if self.token != 0 || self.escape_edges == 1 || CALLBACK.with(|slot| slot.get().0 != 0) {
            Err(Error::ModalPending)
        } else {
            Ok(())
        }
    }
    /// When an epoch is revoked, submitted callbacks still need definite outcomes; do not discard them as "not sent".
    pub fn check_revoked_control(&mut self) -> Result<()> {
        self.check_result()?;
        if self.token != 0 {
            return Err(Error::DeliveryFailed);
        }
        Ok(())
    }
    pub fn refresh(&mut self) {
        if self.token == 0 {
            return;
        }
        // PeekMessage invokes sending-thread callbacks; do not consume posted messages for other components on this thread.
        unsafe {
            let _ = PeekMessageW(&mut MSG::default(), None, 0, 0, PM_NOREMOVE);
        }
        if CALLBACK.with(|slot| slot.get() == (self.token, true)) {
            self.rejected |= self
                .publication
                .as_ref()
                .is_some_and(|request| !request.finished());
            self.publication = None;
            CALLBACK.with(|slot| slot.set((0, false)));
            self.token = 0;
            // The original menu may close earlier for other reasons; the up for a posted Escape down must still travel through the same queue.
            if self.escape_edges != 1 {
                self.clear();
            }
        }
    }
    pub fn pending(&mut self) -> bool {
        self.refresh();
        self.in_flight()
    }
    /// Routing/receipt decisions may read only the latest explicit observation; do not pump callbacks again after checking results.
    pub fn in_flight(&self) -> bool {
        self.token != 0
    }
    pub fn cancel(&mut self, may_signal: bool) -> bool {
        self.publication = None;
        // If the original popup display instance has changed generations, do not send WM_CANCELMODE to the new menu; observe unknown callbacks within the original budget.
        // Revoking control must end menus initiated by this slot; merely waiting for a still-running modal pump is insufficient.
        // Posted cancellation preserves existing Escape-edge order without synchronous blocking or simulated keys/clicks.
        if self.pending() && may_signal {
            if let Some(target) = &self.destination {
                let result = target
                    .preflight()
                    .map_err(Error::Destination)
                    .and_then(|_| {
                        unsafe {
                            PostMessageW(Some(target.hwnd()), WM_CANCELMODE, WPARAM(0), LPARAM(0))
                        }
                        .map_err(|_| Error::DeliveryFailed)
                    });
                if let Err(error) = result {
                    eprintln!("native-live-modal-cancel: {error:?}");
                }
            }
        }
        // Cancellation-message admission does not mean completion. Continue observing the original callback only inside the worker for at most about 50ms;
        // applications that ignore cancellation retain failure status, without retries or treating still-open menus as complete.
        let deadline = Instant::now() + Duration::from_millis(50);
        while self.pending() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        let unknown = self.pending();
        self.clear();
        unknown
    }
    /// Borrow the route without observing callbacks or cloning its process handles.
    pub fn escape_target(&self) -> Option<&Destination> {
        if self.in_flight() || self.escape_edges == 1 {
            self.destination.as_ref()
        } else {
            None
        }
    }
    pub fn escape_needs_release(&self, target: &Destination) -> bool {
        self.escape_edges == 1
            && self
                .destination
                .as_ref()
                .is_some_and(|original| original.same_target(target))
    }
    fn clear(&mut self) {
        self.publication = None;
        CALLBACK.with(|slot| {
            if slot.get().0 == self.token {
                slot.set((0, false));
            }
        });
        self.token = 0;
        self.escape_edges = 0;
        self.destination = None;
    }
    /// Modal menus handle Escape in GetMessage/PeekMessage loops, not ordinary window procedures.
    /// Deduplicate while held; after full release, the next Escape may close the next menu level. Admission does not mean processing is complete.
    pub fn post_escape(&mut self, target: &Destination, packet: Message) -> Result<bool> {
        if (self.token == 0 && self.escape_edges != 1)
            || self
                .destination
                .as_ref()
                .is_none_or(|original| !original.same_target(target))
            || packet.wp.0 != VK_ESCAPE.0 as usize
            || !matches!(packet.id, WM_KEYDOWN | WM_KEYUP)
        {
            return Err(Error::InvalidPayload);
        }
        target.preflight().map_err(Error::Destination)?;
        if target.thread() == unsafe { GetCurrentThreadId() } {
            return Err(Error::SameThread);
        }
        if unsafe { IsIconic(target.root().hwnd()) }.as_bool() {
            return Err(Error::SourceTransition);
        }
        let edge = if packet.id == WM_KEYDOWN { 1 } else { 2 };
        let next_press = edge == 1 && self.escape_edges == 3;
        if self.escape_edges & edge != 0 && !next_press {
            return Ok(false);
        }
        unsafe { PostMessageW(Some(target.hwnd()), packet.id, packet.wp, packet.lp) }
            .map_err(|_| Error::DeliveryFailed)?;
        self.escape_edges = if next_press {
            1
        } else {
            self.escape_edges | edge
        };
        if self.token == 0 {
            self.clear();
        }
        Ok(true)
    }
}
impl Drop for Modal {
    fn drop(&mut self) {
        self.clear();
    }
}

/// After API admission, wait at most the original 250ms budget; delivery() explicitly returns Pending if no callback arrives.
pub fn button_modal(target: &Destination, packet: Message, modal: &mut Modal) -> Result<()> {
    if !matches!(
        packet.id,
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK | WM_LBUTTONUP | WM_RBUTTONUP
    ) {
        return Err(Error::InvalidPayload);
    }
    submit(target, packet, modal)
}

/// Only for compensation calls with validated button class, parent-child relationship, and source identity; arbitrary WM_COMMAND is not accepted.
pub fn button_command(
    parent: &Destination,
    child: &Destination,
    id: u16,
    modal: &mut Modal,
) -> Result<()> {
    submit(
        parent,
        Message {
            id: WM_COMMAND,
            wp: WPARAM(usize::from(id)), // BN_CLICKED's high word is zero.
            lp: LPARAM(child.hwnd().0 as isize),
            moving: false,
            releasing: false,
        },
        modal,
    )
}

fn submit(target: &Destination, packet: Message, modal: &mut Modal) -> Result<()> {
    modal.check_result()?;
    modal.require_idle()?;
    if target.thread() == unsafe { GetCurrentThreadId() } {
        return Err(Error::SameThread);
    }
    target.preflight().map_err(Error::Destination)?;
    if unsafe { IsIconic(target.root().hwnd()) }.as_bool() {
        return Err(Error::SourceTransition);
    }
    let _dpi = input_coordinates::Dpi::enter(target.hwnd()).map_err(|_| Error::Dpi)?;
    let token = NEXT
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| Error::ModalPending)?;
    CALLBACK.with(|slot| slot.set((token, false)));
    modal.token = token;
    modal.destination = Some(target.clone());
    let wire = (|| {
        if let Some(owner) = modal.owner.filter(|_| {
            otter_live_hover_hook::native_protocol::valid_packet(
                packet.id,
                packet.wp.0,
                packet.lp.0,
            ) && target.thread() == unsafe { GetWindowThreadProcessId(target.root().hwnd(), None) }
        }) {
            let publication = native_request::Publication::publish(
                owner,
                target,
                packet,
                token,
                modal.popup_scope,
            )?;
            let message = publication.message()?;
            modal.publication = Some(publication);
            Ok(message)
        } else {
            Ok(packet)
        }
    })();
    let packet = match wire {
        Ok(packet) => packet,
        Err(error) => {
            modal.clear();
            return Err(error);
        }
    };
    if unsafe {
        SendMessageCallbackW(
            target.hwnd(),
            packet.id,
            packet.wp,
            packet.lp,
            Some(completed),
            token,
        )
    }
    .is_err()
    {
        modal.clear();
        return Err(Error::DeliveryFailed);
    }
    let deadline = Instant::now() + Duration::from_millis(250);
    while modal.pending() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    modal.check_result()
}

#[cfg(test)]
#[path = "live_input_modal_admission_tests.rs"]
mod admission_tests;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_callback_cannot_complete_or_clear_a_new_slot() {
        let mut old = Modal::default();
        old.token = 11;
        CALLBACK.with(|slot| slot.set((11, false)));
        old.clear();
        let mut current = Modal::default();
        current.token = 12;
        CALLBACK.with(|slot| slot.set((12, false)));
        unsafe {
            completed(HWND::default(), WM_RBUTTONUP, 11, LRESULT(0));
        }
        drop(old);
        assert_eq!(CALLBACK.with(Cell::get), (12, false));
        unsafe {
            completed(HWND::default(), WM_RBUTTONUP, 12, LRESULT(0));
        }
        assert_eq!(CALLBACK.with(Cell::get), (12, true));
        drop(current);
        assert_eq!(CALLBACK.with(Cell::get), (0, false));
    }
    #[test]
    fn a_retired_display_does_not_receive_cancelmode_or_fake_callback_completion() {
        struct Window(HWND);
        impl Drop for Window {
            fn drop(&mut self) {
                unsafe {
                    DestroyWindow(self.0).unwrap();
                }
            }
        }
        let window = Window(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                windows::core::w!("STATIC"),
                windows::core::w!(""),
                WS_POPUP,
                -30000,
                -30000,
                20,
                20,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        });
        let root = Target::bind(window.0).unwrap();
        let mut modal = Modal::default();
        modal.token = 9001;
        modal.destination = Some(Destination::bind(&root, window.0).unwrap());
        CALLBACK.with(|slot| slot.set((9001, false)));
        assert!(
            modal.cancel(false),
            "unobserved callback must remain unknown"
        );
        assert_eq!(CALLBACK.with(Cell::get), (0, false));
        assert!(!unsafe {
            PeekMessageW(
                &mut MSG::default(),
                Some(window.0),
                WM_CANCELMODE,
                WM_CANCELMODE,
                PM_REMOVE,
            )
        }
        .as_bool());
    }
}
