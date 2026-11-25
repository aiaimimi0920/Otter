//! 会话级有界 GPU 快照纹理池：仅回收已无消费者引用的同规格纹理，会话关闭即清空。
use super::{Error, Texture};
use std::sync::{atomic::AtomicBool, Arc, Mutex, Weak};
use windows::{
    core::{IUnknown, Interface},
    Win32::Graphics::{Direct3D11::*, Dxgi::Common::DXGI_FORMAT},
};

const MAX_IDLE: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    device: usize,
    width: u32,
    height: u32,
    format: DXGI_FORMAT,
}

pub(super) struct Allocation {
    pub texture: ID3D11Texture2D,
    // 活跃帧与 idle 池传递同一 owner；先释放 COM，再归还字节预算。
    _lease: crate::byte_budget::Lease,
}

#[derive(Default)]
pub(crate) struct State {
    closed: bool,
    key: Option<Key>,
    idle: Vec<Allocation>,
}

#[derive(Clone, Default)]
pub struct Pool(Arc<Mutex<State>>);

#[derive(Clone)]
pub(super) struct Returner {
    state: Weak<Mutex<State>>,
    key: Key,
}

impl Pool {
    pub fn new() -> Self {
        Self::default()
    }

    /// # Safety
    /// 与 `live_texture::snapshot` 相同。
    pub unsafe fn snapshot(
        &self,
        context: &ID3D11DeviceContext,
        source: &ID3D11Texture2D,
        cancel: &AtomicBool,
    ) -> Result<Texture, Error> {
        super::copy_pooled(
            context,
            source,
            None,
            cancel,
            Some(&self.0),
            crate::pixels::reserve,
        )
    }

    /// # Safety
    /// 与 `live_texture::snapshot_region` 相同。
    pub unsafe fn snapshot_region(
        &self,
        context: &ID3D11DeviceContext,
        source: &ID3D11Texture2D,
        region: crate::capture_readback::Region,
        cancel: &AtomicBool,
    ) -> Result<Texture, Error> {
        super::copy_pooled(
            context,
            source,
            Some(region),
            cancel,
            Some(&self.0),
            crate::pixels::reserve,
        )
    }

    pub fn close(&self) {
        let idle = if let Ok(mut state) = self.0.lock() {
            state.closed = true;
            std::mem::take(&mut state.idle)
        } else {
            Vec::new()
        };
        drop(idle);
    }

    #[cfg(test)]
    pub(super) fn idle_len(&self) -> usize {
        self.0.lock().map(|s| s.idle.len()).unwrap_or(0)
    }

    #[cfg(test)]
    pub(super) unsafe fn snapshot_with_budget(
        &self,
        context: &ID3D11DeviceContext,
        source: &ID3D11Texture2D,
        budget: &Arc<crate::byte_budget::Budget>,
    ) -> Result<Texture, Error> {
        super::copy_pooled(
            context,
            source,
            None,
            &AtomicBool::new(false),
            Some(&self.0),
            |bytes| budget.reserve(bytes),
        )
    }
}

pub(super) unsafe fn acquire(
    pool: Option<&Arc<Mutex<State>>>,
    device: &ID3D11Device,
    desc: &D3D11_TEXTURE2D_DESC,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<(Allocation, Option<Returner>), Error> {
    let device_id = device.cast::<IUnknown>().map_err(Error::Api)?.as_raw() as usize;
    let key = Key {
        device: device_id,
        width: desc.Width,
        height: desc.Height,
        format: desc.Format,
    };
    let returner = pool.map(|state| Returner {
        state: Arc::downgrade(state),
        key,
    });
    if let Some(pool) = pool {
        let mut retired = Vec::new();
        let reused = pool.lock().ok().and_then(|mut state| {
            if state.closed {
                return None;
            }
            if state.key != Some(key) {
                retired = std::mem::take(&mut state.idle);
                state.key = Some(key);
            }
            state.idle.pop()
        });
        // 换设备/规格先释放旧 idle，且不在池锁内执行 COM 析构。
        drop(retired);
        if let Some(allocation) = reused {
            return Ok((allocation, returner));
        }
    }
    let lease = reserve(super::bytes(desc)?).ok_or(Error::Budget)?;
    let mut texture = None;
    device
        .CreateTexture2D(desc, None, Some(&mut texture))
        .map_err(Error::Api)?;
    Ok((
        Allocation {
            texture: texture.ok_or(Error::Device)?,
            _lease: lease,
        },
        returner,
    ))
}

pub(super) fn recycle(returner: Returner, allocation: Allocation) {
    let Some(pool) = returner.state.upgrade() else {
        return;
    };
    let keep = if let Ok(mut state) = pool.lock() {
        if !state.closed && state.key == Some(returner.key) && state.idle.len() < MAX_IDLE {
            state.idle.push(allocation);
            None
        } else {
            Some(allocation)
        }
    } else {
        Some(allocation)
    };
    drop(keep);
}
