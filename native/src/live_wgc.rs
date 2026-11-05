//! 持续 WGC 显示器/窗口会话：事件通知、两帧池、同线程 GPU 提取，显式 owner 关闭全部资源。
use super::{device, lifetime, pending, same_size};
use crate::{
    live_frames::Source,
    live_texture::{self, Texture},
};
use std::{
    marker::PhantomData,
    rc::Rc,
    sync::{mpsc::Receiver, Arc},
};
use windows::{
    core::{IInspectable, Interface},
    Foundation::TypedEventHandler,
    Graphics::{Capture::*, SizeInt32},
    Win32::{
        Graphics::{Direct3D11::*, Gdi::HMONITOR},
        System::WinRT::{
            Direct3D11::IDirect3DDxgiInterfaceAccess,
            Graphics::Capture::IGraphicsCaptureItemInterop,
        },
    },
};
#[path = "live_wgc_pool.rs"]
mod pool;
#[path = "live_wgc_resize.rs"]
mod resize;
#[path = "live_wgc_signal.rs"]
mod signal;
#[derive(Debug)]
pub enum Error {
    InvalidMonitor,
    Unsupported,
    Dimensions,
    Budget,
    Resized,
    TargetChanged,
    Cancelled,
    SourceClosed,
    NoConsumers,
    Api(windows::core::Error),
    Texture(live_texture::Error),
    Delivery(crate::live_frames::Error),
}
impl From<windows::core::Error> for Error {
    fn from(value: windows::core::Error) -> Self {
        Self::Api(value)
    }
}
#[derive(Clone)]
pub struct Stop(Arc<signal::Signal>);
impl Stop {
    pub fn cancel(&self) {
        self.0.stop();
    }
}
pub struct Session {
    resources: lifetime::Resources,
    item: GraphicsCaptureItem,
    frame_token: Option<i64>,
    close_token: Option<i64>,
    context: ID3D11DeviceContext,
    winrt: windows::Graphics::DirectX::Direct3D11::IDirect3DDevice,
    signal: Arc<signal::Signal>,
    receiver: Receiver<()>,
    frames: Arc<Source<Texture>>,
    size: SizeInt32,
    region: Option<crate::capture_readback::Region>,
    window_crop: Option<(
        crate::live_window_target::Target,
        crate::live_window_target::region::Crop,
    )>,
    resize_allowed: bool,
    _budget: Option<crate::byte_budget::Lease>,
    // Apartment 最后析构；Rc marker 禁止把线程初始化 owner 移到另一线程。
    _apartment: lifetime::Apartment,
    _thread: PhantomData<Rc<()>>,
}
impl Session {
    /// 仅用于用户明确授权的显示器 Live 请求；调用线程持有并最终析构整个会话。
    /// 启动前必须已有消费者；调用者负责隐藏宿主/捕获排除及目标身份授权。
    pub fn monitor(monitor: HMONITOR, frames: Arc<Source<Texture>>) -> Result<Self, Error> {
        Self::start_monitor(monitor, None, frames)
    }
    pub fn monitor_region(
        monitor: HMONITOR,
        region: crate::capture_readback::Region,
        frames: Arc<Source<Texture>>,
    ) -> Result<Self, Error> {
        Self::start_monitor(monitor, Some(region), frames)
    }
    /// 绑定一个已获授权的窗口，不退化为桌面区域；移动跟随由 WGC item 负责。
    /// 调用方在后续 next_frame_checked 中继续复核 Target；窗口尺寸变化按预算重建帧池。
    pub fn window(
        target: &crate::live_window_target::Target,
        frames: Arc<Source<Texture>>,
    ) -> Result<Self, Error> {
        Self::window_region(target, None, frames)
    }
    pub fn window_region(
        target: &crate::live_window_target::Target,
        crop: Option<crate::live_window_target::region::Crop>,
        frames: Arc<Source<Texture>>,
    ) -> Result<Self, Error> {
        target.validate().map_err(|_| Error::TargetChanged)?;
        let mut session = Self::start(crop.map(|crop| crop.pixels()), true, frames, || {
            target.validate().map_err(|_| Error::TargetChanged)?;
            let interop =
                windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
            let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(target.hwnd())? };
            target.validate().map_err(|_| Error::TargetChanged)?;
            if let Some(crop) = crop {
                let size = item.Size()?;
                let bounds = target.frame_bounds().map_err(|_| Error::TargetChanged)?;
                if !crop.matches_texture(bounds, [size.Width as u32, size.Height as u32])? {
                    return Err(Error::Resized);
                }
            }
            Ok(item)
        })?;
        session.window_crop = crop.map(|crop| (target.clone(), crop));
        Ok(session)
    }
    fn start_monitor(
        monitor: HMONITOR,
        region: Option<crate::capture_readback::Region>,
        frames: Arc<Source<Texture>>,
    ) -> Result<Self, Error> {
        if monitor.0.is_null() {
            return Err(Error::InvalidMonitor);
        }
        Self::start(region, false, frames, || {
            let interop =
                windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
            Ok(unsafe { interop.CreateForMonitor(monitor)? })
        })
    }
    fn start(
        region: Option<crate::capture_readback::Region>,
        resize_allowed: bool,
        frames: Arc<Source<Texture>>,
        create: impl FnOnce() -> Result<GraphicsCaptureItem, Error>,
    ) -> Result<Self, Error> {
        if !frames.has_consumers() {
            return Err(Error::NoConsumers);
        }
        let apartment = lifetime::Apartment::new()?;
        if !GraphicsCaptureSession::IsSupported()? {
            return Err(Error::Unsupported);
        }
        let item = create()?;
        let size = item.Size()?;
        pool_bytes(size)?;
        if let Some(region) = region {
            live_texture::region_box(region, size.Width as u32, size.Height as u32)
                .map_err(Error::Texture)?;
        }
        let budget = crate::pixels::reserve(pool_bytes(size)?).ok_or(Error::Budget)?;
        let (_device, context, winrt) = device()?;
        let (signal, receiver) = signal::Signal::new();
        let idle = signal.clone();
        frames
            .on_idle(Arc::new(move || idle.stop()))
            .map_err(Error::Delivery)?;
        let mut owner = Self {
            resources: lifetime::Resources::default(),
            item,
            frame_token: None,
            close_token: None,
            context,
            winrt,
            signal,
            receiver,
            frames,
            size,
            region,
            window_crop: None,
            resize_allowed,
            _budget: Some(budget),
            _apartment: apartment,
            _thread: PhantomData,
        };
        // MTA 已移交 owner；先释放局部 COM 引用，保证后续启动失败时 Apartment 最后退出。
        drop(_device);
        let wake = owner.signal.clone();
        owner.close_token = Some(owner.item.Closed(&TypedEventHandler::<
            GraphicsCaptureItem,
            IInspectable,
        >::new(move |_, _| {
            wake.source_closed();
            Ok(())
        }))?);
        pool::start(
            &mut owner.resources,
            &mut owner.frame_token,
            &owner.winrt,
            &owner.item,
            &owner.signal,
            size,
        )?;
        Ok(owner)
    }
    pub fn stop_handle(&self) -> Stop {
        Stop(self.signal.clone())
    }
    /// 在会话 owner 线程等待并交付下一帧；不在 UI 线程调用。任何终止错误关闭此会话。
    pub fn next_frame(&mut self) -> Result<u64, Error> {
        self.next_frame_checked(|| true)
    }
    /// 调用者在 GPU 复制前后复核目标身份；未通过的快照不得进入共享交付源。
    pub fn next_frame_checked(&mut self, mut validate: impl FnMut() -> bool) -> Result<u64, Error> {
        let result = self.next_inner(&mut validate);
        if result.is_err() {
            self.close();
        }
        result
    }
    fn next_inner(&mut self, validate: &mut dyn FnMut() -> bool) -> Result<u64, Error> {
        loop {
            if !self.frames.has_consumers() {
                return Err(Error::NoConsumers);
            }
            self.signal.wait(&self.receiver)?;
            let pool = self.resources.pool.as_ref().ok_or(Error::Cancelled)?;
            let Some(frame) =
                latest_frame(|| Ok(pending(pool.TryGetNextFrame())?.map(lifetime::Frame)))?
            else {
                continue;
            };
            self.signal.check()?;
            if !validate() {
                return Err(Error::TargetChanged);
            }
            let content = frame.0.ContentSize()?;
            if let Some((target, crop)) = &self.window_crop {
                // 最小化不重建零池；resize 的旧帧不交付为另一个偏移的画面。
                if unsafe { windows::Win32::UI::WindowsAndMessaging::IsIconic(target.hwnd()) }
                    .as_bool()
                    || content.Width == 0
                    || content.Height == 0
                {
                    continue;
                }
                if content.Width < 0 || content.Height < 0 {
                    return Err(Error::Dimensions);
                }
                let bounds = target.frame_bounds().map_err(|_| Error::TargetChanged)?;
                if !crop.matches_texture(bounds, [content.Width as u32, content.Height as u32])? {
                    continue;
                }
            }
            match resize::plan(self.resize_allowed, self.size, content)? {
                resize::Plan::Same => {}
                resize::Plan::Suspend => continue,
                resize::Plan::Recreate => {
                    // 旧帧先 Close，再重建池；这张尺寸变化帧不作为可交付快照使用。
                    drop(frame);
                    self.recreate(content)?;
                    self.signal.check()?;
                    if !validate() {
                        return Err(Error::TargetChanged);
                    }
                    continue;
                }
            }
            let access: IDirect3DDxgiInterfaceAccess = frame.0.Surface()?.cast()?;
            let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            unsafe {
                texture.GetDesc(&mut desc);
            }
            if desc.Width != self.size.Width as u32 || desc.Height != self.size.Height as u32 {
                return Err(Error::Resized);
            }
            // 仅本线程提取/复制，D3D 多线程保护由共享设备工厂开启；Frame guard 先于下一轮归还。
            let prepared = snapshot_checked(validate, || unsafe {
                match self.region {
                    Some(region) => live_texture::snapshot_region(
                        &self.context,
                        &texture,
                        region,
                        &self.signal.cancel,
                    ),
                    None => live_texture::snapshot(&self.context, &texture, &self.signal.cancel),
                }
                .map_err(|error| match error {
                    live_texture::Error::Cancelled => {
                        self.signal.check().err().unwrap_or(Error::Cancelled)
                    }
                    other => Error::Texture(other),
                })
            })?;
            self.signal.check()?;
            return self.frames.publish(prepared).map_err(|error| match error {
                crate::live_frames::Error::NoConsumers => Error::NoConsumers,
                other => Error::Delivery(other),
            });
        }
    }
    pub fn close(&mut self) {
        self.signal.stop();
        if let (Some(pool), Some(token)) = (&self.resources.pool, self.frame_token.take()) {
            if let Err(error) = pool.RemoveFrameArrived(token) {
                eprintln!("native-live-unsubscribe-frame: {error}");
            }
        }
        if let Some(token) = self.close_token.take() {
            if let Err(error) = self.item.RemoveClosed(token) {
                eprintln!("native-live-unsubscribe-close: {error}");
            }
        }
        if let Some(session) = self.resources.session.take() {
            if let Err(error) = session.Close() {
                eprintln!("native-live-close-session: {error}");
            }
        }
        if let Some(pool) = self.resources.pool.take() {
            if let Err(error) = pool.Close() {
                eprintln!("native-live-close-pool: {error}");
            }
        }
        self.frames.close();
        drop(self._budget.take());
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}
fn pool_bytes(size: SizeInt32) -> Result<usize, Error> {
    if size.Width <= 0 || size.Height <= 0 {
        return Err(Error::Dimensions);
    }
    crate::viewport::checked_image_bytes(size.Width as u32, size.Height as u32)
        .and_then(|bytes| bytes.checked_mul(2))
        .ok_or(Error::Dimensions)
}
fn latest_frame<T>(mut next: impl FnMut() -> Result<Option<T>, Error>) -> Result<Option<T>, Error> {
    let Some(first) = next()? else {
        return Ok(None);
    };
    // 合并通知时最多消费两池槽，不无限追赶生产者；旧 Frame guard 自动归还。
    Ok(Some(next()?.unwrap_or(first)))
}
fn snapshot_checked<T>(
    validate: &mut dyn FnMut() -> bool,
    prepare: impl FnOnce() -> Result<T, Error>,
) -> Result<T, Error> {
    if !validate() {
        return Err(Error::TargetChanged);
    }
    let frame = prepare()?;
    if !validate() {
        return Err(Error::TargetChanged);
    }
    Ok(frame)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_target_never_returns_prepared_frame_and_releases_its_owner() {
        assert!(matches!(
            snapshot_checked::<u8>(&mut || false, || panic!("stale capture")),
            Err(Error::TargetChanged)
        ));
        let resource = Arc::new(7u8);
        let weak = Arc::downgrade(&resource);
        let mut calls = 0;
        let result = snapshot_checked(
            &mut || {
                calls += 1;
                calls == 1
            },
            || Ok(resource),
        );
        assert!(matches!(result, Err(Error::TargetChanged)));
        assert!(weak.upgrade().is_none());
        assert_eq!(calls, 2);
        assert_eq!(snapshot_checked(&mut || true, || Ok(9u8)).unwrap(), 9);
    }
    #[test]
    fn coalesced_notification_takes_latest_of_two_slots_without_unbounded_drain() {
        let mut queue = std::collections::VecDeque::from([1, 2, 3]);
        assert_eq!(latest_frame(|| Ok(queue.pop_front())).unwrap(), Some(2));
        assert_eq!(queue.len(), 1);
        assert_eq!(latest_frame(|| Ok(queue.pop_front())).unwrap(), Some(3));
        let mut calls = 0;
        assert_eq!(
            latest_frame::<u8>(|| {
                calls += 1;
                Ok(None)
            })
            .unwrap(),
            None
        );
        assert_eq!(calls, 1);
    }
    #[test]
    fn invalid_monitor_and_missing_demand_never_start_capture() {
        let frames = Arc::new(Source::new());
        assert!(matches!(
            Session::monitor(HMONITOR::default(), frames.clone()),
            Err(Error::InvalidMonitor)
        ));
        assert!(matches!(
            Session::monitor(HMONITOR(std::ptr::dangling_mut()), frames),
            Err(Error::NoConsumers)
        ));
        assert_eq!(
            pool_bytes(SizeInt32 {
                Width: 100,
                Height: 20
            })
            .unwrap(),
            16000
        );
        for size in [
            SizeInt32 {
                Width: 0,
                Height: 1,
            },
            SizeInt32 {
                Width: -1,
                Height: 1,
            },
            SizeInt32 {
                Width: i32::MAX,
                Height: i32::MAX,
            },
        ] {
            assert!(pool_bytes(size).is_err());
        }
    }
}
