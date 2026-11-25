//! Actual PNGs exceeding old ordinary-JSON limits still use independent v2's 6MiB limit, without relaxing traditional requests.
use super::*;
pub(super) fn large_image() -> Image {
    let (width, height) = (1024, 512);
    let mut rgba = Vec::with_capacity(width * height * 4);
    let mut seed = 0x12345678u32;
    for _ in 0..width * height {
        for _ in 0..3 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            rgba.push(seed as u8);
        }
        rgba.push(255);
    }
    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&rgba)
            .unwrap();
    }
    let png: crate::pixels::Pixels = encoded.into();
    Image::from_png(&png, width as u32, height as u32, &AtomicBool::new(false)).unwrap()
}
#[test]
fn actual_large_formal_png_roundtrip_is_bounded_and_independently_decoded() {
    let image = large_image();
    let (width, height) = (image.width(), image.height());
    let png = image.png(&AtomicBool::new(false)).unwrap();
    let envelope = envelope(&image);
    let operation = Operation::Create {
        unit: "unit-1",
        kind: "sticker",
        image: &image,
    };
    let body = operation.body().unwrap();
    assert!(body.len() > 1024 * 1024 && body.len() < MAX_WIRE);
    drop(body);
    let listener = listener();
    let target = target(&listener);
    let value = wire(&envelope, Some(&image), 1, image.digest(), None);
    let peer = std::thread::spawn(move || {
        bootstrap(&listener, SOURCE);
        let body = respond(&listener, "POST /v1/projections/v2/create", true, &value);
        assert_eq!(body["snapshot"], value["snapshot"]);
    });
    let credential = session(AUTHORITY, now().unwrap() + 120_000);
    let cancel = Cancel::new();
    let account = Account::open(target, &credential, &cancel).unwrap();
    let State::Content(content) = account.execute(operation).unwrap() else {
        panic!()
    };
    let received = content.image.unwrap();
    assert_eq!(
        (received.width(), received.height()),
        (width as u32, height as u32)
    );
    assert_eq!(&*received.png(&AtomicBool::new(false)).unwrap(), &*png);
    peer.join().unwrap();
}
#[test]
fn account_body_and_response_budgets_do_not_authorize_unbounded_or_other_requests() {
    let listener = listener();
    let endpoint = http::Endpoint::new(target(&listener)).unwrap();
    let credential = session(AUTHORITY, now().unwrap() + 120_000);
    let cancel = Cancel::new();
    let oversized = vec![b' '; MAX_WIRE + 1];
    assert!(matches!(
        endpoint.request(
            "POST",
            "/v1/projections/v2/create",
            &oversized,
            Some(&credential),
            &cancel,
            1024
        ),
        Err(Error::Input)
    ));
    assert!(matches!(
        endpoint.request("GET", "/v1/otter/identity", b"{}", None, &cancel, 1024),
        Err(Error::Input)
    ));
    assert!(matches!(
        endpoint.request(
            "POST",
            "/v1/projections/v2/read",
            b"{}",
            Some(&credential),
            &cancel,
            MAX_WIRE + 1
        ),
        Err(Error::Input)
    ));
    assert!(listener.accept().is_err());
}
