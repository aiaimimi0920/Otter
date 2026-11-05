//! Live 消费者到 HWND 的有界事件桥；只投递连接代次，不传裸指针或图像字节。
use crate::live_frames::{Consumer, Delivery};
use std::{
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        System::Threading::{GetCurrentProcessId, GetCurrentThreadId},
        UI::WindowsAndMessaging::*,
    },
};
pub const MESSAGE: u32 = WM_APP + 0x350;
static NEXT: AtomicUsize = AtomicUsize::new(1);
pub struct WindowSignal<T> {
    consumer: Consumer<T>,
    hwnd: HWND,
    token: usize,
    _thread: PhantomData<Rc<()>>,
}
impl<T> WindowSignal<T> {
    /// 投递失败累计数；不忙重试，下一次帧变化重新尝试。宿主可在已有事件中报告此状态。
    pub fn notification_failures(&self) -> u64 {
        self.consumer.notification_failures()
    }
    /// 已有绘制/宿主事件中的补偿读取，不创建轮询线程或定时器。
    pub(super) fn refresh(
        &mut self,
    ) -> std::result::Result<Option<Delivery<T>>, crate::live_frames::Error> {
        self.consumer.try_next()
    }
    /// 在目标 HWND 的所属线程绑定，owner 必须随窗口销毁；旧排队消息由新连接代次拒绝。
    pub fn bind(hwnd: HWND, mut consumer: Consumer<T>) -> Result<Self> {
        unsafe {
            let mut process = 0;
            let thread = GetWindowThreadProcessId(hwnd, Some(&mut process));
            if !IsWindow(Some(hwnd)).as_bool()
                || process != GetCurrentProcessId()
                || thread != GetCurrentThreadId()
            {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Live notification requires an owned HWND on this thread",
                ));
            }
        }
        let token = NEXT
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| Error::new(E_FAIL, "Live notification identity exhausted"))?;
        let address = hwnd.0 as usize;
        consumer
            .set_waker(move || unsafe {
                PostMessageW(
                    Some(HWND(address as *mut _)),
                    MESSAGE,
                    WPARAM(token),
                    LPARAM(0),
                )
                .is_ok()
            })
            .map_err(|error| {
                Error::new(E_FAIL, format!("Live notification unavailable: {error:?}"))
            })?;
        Ok(Self {
            consumer,
            hwnd,
            token,
            _thread: PhantomData,
        })
    }
    /// 返回 None 表示不属于此连接；消息只唤醒读取，真实内容始终取自已绑定消费者。
    pub fn receive(
        &mut self,
        hwnd: HWND,
        message: u32,
        wp: WPARAM,
        lp: LPARAM,
    ) -> Option<std::result::Result<Option<Delivery<T>>, crate::live_frames::Error>> {
        if hwnd != self.hwnd || message != MESSAGE || wp.0 != self.token || lp.0 != 0 {
            return None;
        }
        Some(self.consumer.try_next())
    }
}
#[cfg(test)]
#[path = "composition_live_notify_tests.rs"]
mod tests;
