use super::*;
use crate::{
    byte_budget::Budget,
    live_relay_frame::Metadata,
    live_relay_receive::{PendingFrame, Profile},
};
use windows::Win32::{
    Foundation::HGLOBAL,
    Graphics::Imaging::*,
    System::Com::{StructuredStorage::CreateStreamOnHGlobal, *},
};
fn received(codec: Codec, width: u32, height: u32, bytes: &[u8]) -> Frame {
    let metadata = Metadata {
        epoch: 1,
        frame_id: 1,
        capture_timestamp_ms: 1,
        encode_timestamp_ms: 1,
        width,
        height,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec,
    };
    let mut pending = PendingFrame::begin(
        &metadata.encode_header(bytes.len()).unwrap(),
        Profile::H264,
        &Budget::new(bytes.len()),
    )
    .unwrap();
    pending.push(bytes).unwrap();
    pending.finish().unwrap()
}
fn jpeg() -> Vec<u8> {
    unsafe {
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let stream = CreateStreamOnHGlobal(HGLOBAL::default(), true).unwrap();
        let encoder = factory
            .CreateEncoder(&GUID_ContainerFormatJpeg, std::ptr::null())
            .unwrap();
        encoder
            .Initialize(&stream, WICBitmapEncoderNoCache)
            .unwrap();
        let mut frame = None;
        encoder
            .CreateNewFrame(&mut frame, std::ptr::null_mut())
            .unwrap();
        let frame = frame.unwrap();
        frame.Initialize(None).unwrap();
        frame.SetSize(2, 2).unwrap();
        let mut format = GUID_WICPixelFormat24bppBGR;
        frame.SetPixelFormat(&mut format).unwrap();
        assert_eq!(format, GUID_WICPixelFormat24bppBGR);
        frame
            .WritePixels(2, 6, &[0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0, 255])
            .unwrap();
        frame.Commit().unwrap();
        encoder.Commit().unwrap();
        let mut stat = STATSTG::default();
        stream.Stat(&mut stat, STATFLAG_NONAME).unwrap();
        assert!(stat.cbSize < 16384);
        let mut bytes = vec![0; stat.cbSize as usize];
        stream.Seek(0, STREAM_SEEK_SET, None).unwrap();
        let mut read = 0;
        stream
            .Read(
                bytes.as_mut_ptr().cast(),
                bytes.len() as u32,
                Some(&mut read),
            )
            .ok()
            .unwrap();
        assert_eq!(read as usize, bytes.len());
        bytes
    }
}
#[test]
fn actual_wic_jpeg_decodes_exact_dimensions_and_rejects_lies_and_corruption() {
    let _apartment = super::super::Apartment::new().unwrap();
    let bytes = jpeg();
    let image = decode(&received(Codec::Jpeg, 2, 2, &bytes)).unwrap();
    assert_eq!((image.width, image.height), (2, 2));
    for pixel in image.pixels.chunks_exact(4) {
        assert!(pixel[0] < 5 && pixel[1] < 5 && pixel[2] > 250);
        assert_eq!(pixel[3], 255);
    }
    assert!(decode(&received(Codec::Jpeg, 1, 2, &bytes)).is_err());
    assert!(decode(&received(Codec::Jpeg, 2, 2, &[0xff, 0xd8, 0, 0xff, 0xd9])).is_err());
    assert!(decode(&received(Codec::Jpeg, 2, 2, &bytes[..bytes.len() - 1])).is_err());
}
#[test]
fn raw_frame_is_premultiplied_and_video_is_not_treated_as_an_image() {
    let image = decode(&received(
        Codec::RawBgra,
        2,
        1,
        &[200, 100, 50, 128, 255, 255, 255, 0],
    ))
    .unwrap();
    assert_eq!(&*image.pixels, &[100, 50, 25, 128, 0, 0, 0, 0]);
    assert!(decode(&received(Codec::H264, 2, 2, &[0, 0, 0, 1])).is_err());
}
#[test]
fn static_decoders_publish_the_same_pixel_storage_to_viewer_cache() {
    use crate::live_relay_viewer_frames::{Cursor, ViewerPixels};
    let _apartment = super::super::Apartment::new().unwrap();
    for frame in [
        received(Codec::Jpeg, 2, 2, &jpeg()),
        received(Codec::RawBgra, 2, 2, &[255; 16]),
    ] {
        let decoded = decode_pixels(&frame).unwrap();
        assert_eq!(decoded.metadata().codec, Codec::RawBgra);
        let storage = decoded.pixels().clone();
        let cache = ViewerPixels::new(1).unwrap();
        cache.commit(decoded, false).unwrap();
        let output = cache
            .take(Cursor {
                epoch: 1,
                frame_id: 1,
            })
            .unwrap()
            .unwrap();
        assert!(storage.same_storage(output.pixels()));
        assert_eq!(
            output.metadata().capture_timestamp_ms,
            frame.metadata().capture_timestamp_ms
        );
    }
}
