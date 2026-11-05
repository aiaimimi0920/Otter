//! D3D12 有预算资源、上传/读回映射与 barrier；临时 union COM clone 显式释放。
use super::*;
use crate::byte_budget::Lease;
use std::{ffi::c_void, mem::ManuallyDrop};

pub(super) struct Resource {
    pub value: ID3D12Resource,
    pub footprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    pub bytes: usize,
    _lease: Lease,
}
impl Resource {
    pub fn texture(device: &ID3D12Device, width: u32, height: u32, target: bool) -> Result<Self> {
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Width: width as u64,
            Height: height,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: if target {
                D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET
            } else {
                D3D12_RESOURCE_FLAG_NONE
            },
            ..Default::default()
        };
        let mut footprint = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
        let mut bytes = 0;
        unsafe {
            device.GetCopyableFootprints(
                &desc,
                0,
                1,
                0,
                Some(&mut footprint),
                None,
                None,
                Some(&mut bytes),
            );
        }
        let value = Self::create(
            device,
            &desc,
            D3D12_HEAP_TYPE_DEFAULT,
            if target {
                D3D12_RESOURCE_STATE_RENDER_TARGET
            } else {
                D3D12_RESOURCE_STATE_COPY_DEST
            },
        )?;
        Ok(Self {
            value: value.0,
            _lease: value.1,
            footprint,
            bytes: usize::try_from(bytes).map_err(|_| Error::Budget)?,
        })
    }
    pub fn buffer(device: &ID3D12Device, bytes: usize, readback: bool) -> Result<Self> {
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Width: bytes as u64,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            ..Default::default()
        };
        let (value, lease) = Self::create(
            device,
            &desc,
            if readback {
                D3D12_HEAP_TYPE_READBACK
            } else {
                D3D12_HEAP_TYPE_UPLOAD
            },
            if readback {
                D3D12_RESOURCE_STATE_COPY_DEST
            } else {
                D3D12_RESOURCE_STATE_GENERIC_READ
            },
        )?;
        Ok(Self {
            value,
            _lease: lease,
            footprint: Default::default(),
            bytes,
        })
    }
    fn create(
        device: &ID3D12Device,
        desc: &D3D12_RESOURCE_DESC,
        kind: D3D12_HEAP_TYPE,
        state: D3D12_RESOURCE_STATES,
    ) -> Result<(ID3D12Resource, Lease)> {
        let allocation = unsafe { device.GetResourceAllocationInfo(0, std::slice::from_ref(desc)) };
        let bytes = usize::try_from(allocation.SizeInBytes).map_err(|_| Error::Budget)?;
        if bytes == 0 || allocation.SizeInBytes == u64::MAX {
            return Err(Error::Budget);
        }
        let lease = crate::pixels::reserve(bytes).ok_or(Error::Budget)?;
        let mut resource = None;
        unsafe {
            device.CreateCommittedResource(
                &D3D12_HEAP_PROPERTIES {
                    Type: kind,
                    CreationNodeMask: 1,
                    VisibleNodeMask: 1,
                    ..Default::default()
                },
                D3D12_HEAP_FLAG_NONE,
                desc,
                state,
                None,
                &mut resource,
            )
        }
        .map_err(|e| api(Stage::Resource, e))?;
        Ok((resource.ok_or(Error::Unsupported)?, lease))
    }
    pub fn upload(
        &self,
        pixels: &[u8],
        footprint: Option<D3D12_PLACED_SUBRESOURCE_FOOTPRINT>,
        stopped: &impl Fn() -> bool,
        deadline: Instant,
    ) -> Result<()> {
        let mapped = Mapping::new(&self.value, false, self.bytes)?;
        match footprint {
            Some(layout) => {
                let stride = layout.Footprint.Width as usize * 4;
                for (y, row) in pixels.chunks_exact(stride).enumerate() {
                    check(stopped, deadline)?;
                    let offset = layout.Offset as usize + y * layout.Footprint.RowPitch as usize;
                    if offset
                        .checked_add(stride)
                        .filter(|n| *n <= self.bytes)
                        .is_none()
                    {
                        return Err(Error::Layout);
                    }
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            row.as_ptr(),
                            mapped.pointer.cast::<u8>().add(offset),
                            stride,
                        );
                    }
                }
            }
            None => {
                if pixels.len() > self.bytes {
                    return Err(Error::Layout);
                }
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        pixels.as_ptr(),
                        mapped.pointer.cast(),
                        pixels.len(),
                    );
                }
            }
        }
        Ok(())
    }
}
pub(super) struct Mapping<'a> {
    pub pointer: *mut c_void,
    resource: &'a ID3D12Resource,
    read: bool,
}
impl<'a> Mapping<'a> {
    pub fn new(resource: &'a ID3D12Resource, read: bool, bytes: usize) -> Result<Self> {
        let mut pointer = std::ptr::null_mut();
        unsafe {
            resource.Map(
                0,
                Some(&D3D12_RANGE {
                    Begin: 0,
                    End: if read { bytes } else { 0 },
                }),
                Some(&mut pointer),
            )
        }
        .map_err(|e| api(Stage::Readback, e))?;
        let result = Self {
            pointer,
            resource,
            read,
        };
        if result.pointer.is_null() {
            return Err(Error::Layout);
        }
        Ok(result)
    }
}
impl Drop for Mapping<'_> {
    fn drop(&mut self) {
        unsafe {
            self.resource.Unmap(
                0,
                if self.read {
                    Some(&D3D12_RANGE { Begin: 0, End: 0 })
                } else {
                    None
                },
            );
        }
    }
}
pub(super) unsafe fn transition(
    list: &ID3D12GraphicsCommandList,
    resource: &ID3D12Resource,
    from: D3D12_RESOURCE_STATES,
    to: D3D12_RESOURCE_STATES,
) {
    let mut barrier = D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: ManuallyDrop::new(Some(resource.clone())),
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                StateBefore: from,
                StateAfter: to,
            }),
        },
    };
    list.ResourceBarrier(std::slice::from_ref(&barrier));
    ManuallyDrop::drop(&mut (*barrier.Anonymous.Transition).pResource);
}
pub(super) unsafe fn copy(
    list: &ID3D12GraphicsCommandList,
    texture: &ID3D12Resource,
    buffer: &ID3D12Resource,
    footprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    upload: bool,
) {
    let mut image = D3D12_TEXTURE_COPY_LOCATION {
        pResource: ManuallyDrop::new(Some(texture.clone())),
        Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
        Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
            SubresourceIndex: 0,
        },
    };
    let mut linear = D3D12_TEXTURE_COPY_LOCATION {
        pResource: ManuallyDrop::new(Some(buffer.clone())),
        Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
        Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
            PlacedFootprint: footprint,
        },
    };
    if upload {
        list.CopyTextureRegion(&image, 0, 0, 0, &linear, None);
    } else {
        list.CopyTextureRegion(&linear, 0, 0, 0, &image, None);
    }
    ManuallyDrop::drop(&mut image.pResource);
    ManuallyDrop::drop(&mut linear.pResource);
}
