//! 图像服务只引用原mount安装的不可变PNG；不接脚本字节、路径、URL或外来lease。
use super::{Call, Error, Request};
use crate::{byte_budget::Lease, surface_javascript::ImageResource};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
#[path = "art_service_canvas.rs"]
mod canvas;

const MAX_IMAGES: usize = 4;
const MAX_PNG: usize = 4 * 1024 * 1024;
const TOTAL_PNG: usize = 8 * 1024 * 1024;
pub(super) const MAX_REQUEST: usize = 96 * 1024 + 4 * (TOTAL_PNG + 2 * MAX_IMAGES).div_ceil(3);
#[derive(Serialize)]
pub(super) struct Image {
    pub(super) png: String,
}
pub(super) struct Uploads {
    pub(super) images: Vec<Image>,
    pub(super) sources: Vec<(String, [u32; 2])>,
    pub(super) decoded_limit: usize,
    // 编码字符串、序列化正文及HTTP内部副本有界计费；持有至调用结束。
    _budget: Lease,
}
impl Uploads {
    pub(super) fn encode(&self, call: &Request<'_>) -> Result<Vec<u8>, Error> {
        self.encode_for(call, "loom.art.local.service.images.v1")
    }
    pub(super) fn encode_for(
        &self,
        call: &Request<'_>,
        protocol: &'static str,
    ) -> Result<Vec<u8>, Error> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Envelope<'a, 'b> {
            protocol_version: &'static str,
            call: &'a Request<'b>,
            images: &'a [Image],
        }
        serde_json::to_vec(&Envelope {
            protocol_version: protocol,
            call,
            images: &self.images,
        })
        .map_err(|_| Error::Wire)
    }
    fn select(ids: &[String], installed: &[ImageResource]) -> Result<Self, Error> {
        if ids.is_empty() || ids.len() > MAX_IMAGES {
            return Err(Error::Input);
        }
        let mut chosen = Vec::with_capacity(ids.len());
        let mut total = 0usize;
        for (index, id) in ids.iter().enumerate() {
            if ids[..index].contains(id) {
                return Err(Error::Input);
            }
            let image = installed
                .iter()
                .find(|image| &image.id == id)
                .ok_or(Error::Input)?;
            if image.png.is_empty()
                || image.png.len() > MAX_PNG
                || !image.png.starts_with(b"\x89PNG\r\n\x1a\n")
            {
                return Err(Error::Input);
            }
            total = total.checked_add(image.png.len()).ok_or(Error::Limit)?;
            if total > TOTAL_PNG {
                return Err(Error::Limit);
            }
            chosen.push(image);
        }
        let encoded: usize = chosen
            .iter()
            .map(|image| 4 * image.png.len().div_ceil(3))
            .sum();
        let budget = crate::pixels::reserve(4 * (encoded + 96 * 1024)).ok_or(Error::Limit)?;
        Ok(Self {
            decoded_limit: 16 * 1024 * 1024,
            sources: chosen
                .iter()
                .map(|image| (format!("图片 {}", image.id), [image.width, image.height]))
                .collect(),
            images: chosen
                .into_iter()
                .map(|image| Image {
                    png: STANDARD.encode(&*image.png),
                })
                .collect(),
            _budget: budget,
        })
    }
}
impl Call<'_> {
    pub(super) fn attach_canvases(
        &mut self,
        frames: Vec<crate::surface_javascript_wire::canvas::CanvasFrame>,
    ) -> Result<(), Error> {
        if self.submitted || self.images.is_some() {
            return Err(Error::Input);
        }
        self.images = Some(canvas::prepare(frames)?);
        Ok(())
    }
    pub(super) fn attach_images(
        &mut self,
        ids: &[String],
        installed: &[ImageResource],
    ) -> Result<(), Error> {
        if self.submitted || self.images.is_some() {
            return Err(Error::Input);
        }
        self.images = Some(Uploads::select(ids, installed)?);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn art_service_images_selection_rejects_unknown_duplicate_and_oversized_without_upload() {
        let image = crate::surface_javascript_wire::images::tests::resource();
        assert!(Uploads::select(&["unknown".into()], std::slice::from_ref(&image)).is_err());
        assert!(Uploads::select(
            &[image.id.clone(), image.id.clone()],
            std::slice::from_ref(&image)
        )
        .is_err());
        assert!(Uploads::select(&[], std::slice::from_ref(&image)).is_err());
        let mut large = image;
        large.png = vec![0; MAX_PNG + 1].into();
        assert!(Uploads::select(&[large.id.clone()], &[large]).is_err());
    }
}
