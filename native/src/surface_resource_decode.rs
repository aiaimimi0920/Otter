//! worker 内解码已验摘要的字节；COM 不离开本线程，编码缓冲返回前清零释放。
use super::{request::Request, Error, Lease};
use crate::{image_decode::Image, live_relay_http::VerifiedResource};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
pub(super) struct Decoded {
    pub request: Request,
    pub image: Image,
    pub logical_size: [f32; 2],
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
fn now() -> Result<u64, Error> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::Stopped)?
        .as_millis();
    u64::try_from(millis).map_err(|_| Error::Stopped)
}
pub(super) fn decode(resource: VerifiedResource, lease: &Lease) -> Result<Decoded, Error> {
    let (request, bytes) = resource.into_parts();
    let validate = || {
        lease
            .validate(request.binding().origin(), now()?)
            .map_err(|_| Error::Stopped)?;
        request.validate(now()?).map_err(Error::Resource)
    };
    validate()?;
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() }
        .map_err(|error| Error::Decode(error.code().0))?;
    let _apartment = Apartment;
    // WIC 调用本身不可强制中断；取消在调用前后检查，RGBA 转换逐行检查。
    let decoded = Image::from_surface_sized(&bytes, request.mime(), request.dimensions(), || {
        lease.is_revoked()
    });
    validate()?;
    let (image, logical_size) = decoded.map_err(|error| Error::Decode(error.code().0))?;
    Ok(Decoded {
        request: Request::Resource(request),
        image,
        logical_size,
    })
}
pub(super) fn inline(
    request: super::request::inline::Inline,
    lease: &Lease,
) -> Result<Decoded, Error> {
    let request = Request::Inline(request);
    let validate = || {
        lease
            .validate(request.binding().origin(), now()?)
            .map_err(|_| Error::Stopped)
    };
    validate()?;
    let Request::Inline(source) = &request else {
        unreachable!()
    };
    let bytes = zeroize::Zeroizing::new(
        super::request::data::decode(source.source(), || lease.is_revoked())
            .map_err(Error::Resource)?,
    );
    validate()?;
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() }
        .map_err(|e| Error::Decode(e.code().0))?;
    let _apartment = Apartment;
    let image = Image::from_surface_sized(&bytes, request.mime(), None, || lease.is_revoked());
    validate()?;
    let (image, logical_size) = image.map_err(|e| Error::Decode(e.code().0))?;
    Ok(Decoded {
        request,
        image,
        logical_size,
    })
}
