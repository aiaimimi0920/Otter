//! Continuous WGC monitor/window sessions: event notifications, a two-frame pool, same-thread GPU extraction, and an explicit owner closing all resources.
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
#[path = "live_wgc_drain.rs"]
pub mod drain;
#[path = "live_wgc_pool.rs"]
pub mod pool;
#[path = "live_wgc_resize.rs"]
pub mod resize;
#[path = "live_wgc_signal.rs"]
pub mod signal;
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
    window_target: Option<crate::live_window_target::Target>,
    window_crop: Option<(
        crate::live_window_target::Target,
        crate::live_window_target::region::Crop,
    )>,
    resize_allowed: bool,
    _budget: Option<crate::byte_budget::Lease>,
    // Apartment is destroyed last; an Rc marker prevents moving the thread-initialization owner to another thread.
    _apartment: lifetime::Apartment,
    _thread: PhantomData<Rc<()>>,
}
impl Session {
    /// Only for monitor Live requests explicitly authorized by the user; the calling thread owns and ultimately destroys the entire session.
    /// Consumers must exist before startup; callers own host hiding/capture exclusion and target-identity authorization.
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
    /// Bind an authorized window without degrading to a desktop region; the WGC item tracks movement.
    /// Callers continue validating Target in subsequent next_frame_checked calls; window-size changes rebuild the frame pool within budget.
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
        session.window_target = Some(target.clone());
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
        if !super::factory::supported()? {
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
            window_target: None,
            resize_allowed,
            _budget: Some(budget),
            _apartment: apartment,
            _thread: PhantomData,
        };
        // MTA ownership has transferred; release local COM references first so Apartment exits last if later startup fails.
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
    /// Wait for and deliver the next frame on the session-owner thread, never UI. Any terminal error closes this session.
    pub fn next_frame(&mut self) -> Result<u64, Error> {
        self.next_frame_checked(|| true)
    }
    /// Callers revalidate target identity before and after GPU copies; rejected snapshots must not enter the shared delivery source.
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
            // Both full-window and cropped-window capture retain the last pre-minimization frame, without presenting taskbar-thumbnail dimensions.
            if self.window_target.as_ref().is_some_and(|target| unsafe {
                windows::Win32::UI::WindowsAndMessaging::IsIconic(target.hwnd()).as_bool()
            }) {
                continue;
            }
            if let Some((target, crop)) = &self.window_crop {
                // Minimization does not rebuild a zero-sized pool; old resize frames must not be delivered as a differently offset image.
                if content.Width == 0 || content.Height == 0 {
                    continue;
                }
                if content.Width < 0 || content.Height < 0 {
                    return Err(Error::Dimensions);
                }
                let bounds = match target.frame_bounds() {
                    Ok(bounds) => bounds,
                    // The source may have just minimized before bounds are read; if identity remains valid, wait for restoration rather than treating the transition as a source change.
                    Err(_)
                        if target.validate().is_ok()
                            && unsafe {
                                windows::Win32::UI::WindowsAndMessaging::IsIconic(target.hwnd())
                                    .as_bool()
                            } =>
                    {
                        continue
                    }
                    Err(_) => return Err(Error::TargetChanged),
                };
                if !crop.matches_texture(bounds, [content.Width as u32, content.Height as u32])? {
                    continue;
                }
            }
            match resize::plan(self.resize_allowed, self.size, content)? {
                resize::Plan::Same => {}
                resize::Plan::Suspend => continue,
                resize::Plan::Recreate => {
                    // Close the old frame before rebuilding the pool; this size-change frame is not used as a deliverable snapshot.
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
            // Extract/copy only on this thread; the shared-device factory enables D3D multithread protection, and the Frame guard is returned before the next iteration.
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
        #[cfg(test)]
        eprintln!("native-live-close: unsubscribe");
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
            if let Err(error) = drain::finish(&self.context) {
                // Failure does not mean resources were released; retain diagnostics and continue the original Close rather than dropping the owner to pretend success.
                eprintln!("native-live-close-gpu: {error}");
            }
            #[cfg(test)]
            eprintln!("native-live-close: session");
            if let Err(error) = session.Close() {
                eprintln!("native-live-close-session: {error}");
            }
        }
        if let Some(pool) = self.resources.pool.take() {
            #[cfg(test)]
            eprintln!("native-live-close: pool");
            if let Err(error) = pool.Close() {
                eprintln!("native-live-close-pool: {error}");
            }
        }
        self.frames.close();
        #[cfg(test)]
        eprintln!("native-live-close: complete");
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
    // Consume at most two pool slots when coalescing notifications, without endlessly chasing the producer; old Frame guards return automatically.
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
