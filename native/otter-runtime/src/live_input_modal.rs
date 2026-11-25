//! 本地模态按钮的一槽回调：提交与完成分开，取消在短预算内收取回调。
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
    // 回调只带值 token，绝不保存 Backend/Controller/栈或堆指针。
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
    /// 仅在 check_result 成功后判定空槽；此处不能再次泵回调，否则刚到达的
    /// 拒绝会清掉 token，却越过已经完成的错误检查而放行下一条输入。
    fn require_idle(&self) -> Result<()> {
        if self.token != 0 || self.escape_edges == 1 || CALLBACK.with(|slot| slot.get().0 != 0) {
            Err(Error::ModalPending)
        } else {
            Ok(())
        }
    }
    /// 周期撤销时，已提交的回调仍须有明确结果；不能按“未发送”分类丢掉它。
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
        // PeekMessage 会调用发送线程回调；不消费此线程其他组件的 posted 消息。
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
            // 原菜单可因其他原因先关闭；posted Escape down 的 up 仍须沿同一队列送达。
            if self.escape_edges != 1 {
                self.clear();
            }
        }
    }
    pub fn pending(&mut self) -> bool {
        self.refresh();
        self.in_flight()
    }
    /// 路由/回执判断只能读取最近一次显式观察；不能在结果检查后再次泵回调。
    pub fn in_flight(&self) -> bool {
        self.token != 0
    }
    pub fn cancel(&mut self, may_signal: bool) -> bool {
        self.publication = None;
        // 原 popup 显示实例已换代时不能把 WM_CANCELMODE 投给新菜单；未知回调仍按原预算观察。
        // 撤销控制必须结束由本槽发起的菜单；不能只等待一个仍在运行的模态泵。
        // posted 取消保留既有 Escape 边沿顺序，不同步阻塞，也不模拟按键/点击。
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
        // 取消消息准入不代表完成。仍只在 worker 内观察原回调至多约 50ms；
        // 不响应取消的程序保留失败状态，不重发、不把仍打开的菜单当成完成。
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
    /// 模态菜单在 GetMessage/PeekMessage 循环中处理 Escape，而非普通窗口过程。
    /// 按住期间去重，完整松开后允许下一次 Escape 关闭上一层菜单；准入不等于处理完成。
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

/// API 准入后最多等待原有 250ms 预算；未回调由 delivery() 显式返回 Pending。
pub fn button_modal(target: &Destination, packet: Message, modal: &mut Modal) -> Result<()> {
    if !matches!(
        packet.id,
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK | WM_LBUTTONUP | WM_RBUTTONUP
    ) {
        return Err(Error::InvalidPayload);
    }
    submit(target, packet, modal)
}

/// 仅供已校验按钮类别、父子关系和源身份的补偿调用；不接受任意 WM_COMMAND。
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
            wp: WPARAM(usize::from(id)), // BN_CLICKED 的高字为零。
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
