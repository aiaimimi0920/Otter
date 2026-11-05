//! 普通产品Art投送的隔离接收端；独立PNG oracle，不使用生产合成器或真实设备。
use super::*;
#[path = "projection_oracle.rs"]
pub(crate) mod oracle;
#[derive(Default)]
pub struct State {
    pub enabled: bool,
    pub envelope: Option<Value>,
    pub revision: u64,
    pub digest: String,
    pub reads: usize,
    pub unlinked: bool,
    pub frames: Vec<Vec<u8>>,
}
impl State {
    pub fn respond(&mut self, path: &str, body: &Value) -> (u16, Value) {
        assert!(
            self.enabled,
            "projection must be explicitly enabled in owned fixture"
        );
        if path.ends_with("/targets") {
            assert_eq!(body, &json!({}));
            return (
                200,
                json!({"targets":[{"deviceId":"product-receiver","name":"Product Receiver",
                "policy":"auto","route":"shared_loom"}]}),
            );
        }
        if path.ends_with("/read") && self.envelope.is_none() {
            return (404, json!({"error":{"code":"projection_not_found"}}));
        }
        assert!(
            ["/create", "/read", "/update", "/unlink"]
                .iter()
                .any(|suffix| path.ends_with(suffix)),
            "unexpected sender route: {path}"
        );
        if path.ends_with("/create") {
            assert!(self.envelope.is_none());
            assert_eq!(body["targetDeviceId"], "product-receiver");
            self.envelope = Some(body["envelope"].clone());
            self.accept_frame(body, 1, [255, 0, 0, 255]);
            assert_eq!(
                self.envelope.as_ref().unwrap()["content"]["digest"],
                self.digest
            );
        } else {
            let envelope = self.envelope.as_ref().unwrap();
            assert_eq!(body["projectionId"], envelope["projectionId"]);
            if path.ends_with("/unlink") {
                assert_eq!(self.revision, 2);
                self.unlinked = true;
                return (200, json!({"unlinked":true}));
            }
            if self.unlinked {
                return (410, json!({"error":{"code":"projection_unlinked"}}));
            }
            if path.ends_with("/update") {
                assert_eq!(
                    self.revision, 1,
                    "preview/late output must not publish another update"
                );
                assert_eq!(body["sourceSessionId"], envelope["source"]["sessionId"]);
                assert_eq!(body["priorRevision"], 1);
                assert_eq!(body["revision"], 2);
                assert_eq!(body.as_object().unwrap().len(), 6);
                self.accept_frame(body, 2, [0, 255, 0, 255]);
                assert_eq!(body["digest"], self.digest);
            } else {
                assert!(path.ends_with("/read"));
                self.reads += 1;
                assert!(self.reads < 64);
                assert!(body["knownRevision"].as_u64().unwrap() <= self.revision);
            }
        }
        (
            200,
            json!({"envelope":self.envelope,"revision":self.revision,"digest":self.digest,"linked":true,
            "receiverDeviceId":"product-receiver","receiverUnitId":"remote-art","snapshot":null,
            "delivery":{"targetDeviceId":"product-receiver","status":"accepted"}}),
        )
    }
    fn accept_frame(&mut self, body: &Value, revision: u64, rgba: [u8; 4]) {
        assert!(self.frames.len() < 2);
        assert_eq!(
            (
                body["snapshot"]["width"].as_u64(),
                body["snapshot"]["height"].as_u64()
            ),
            (Some(2), Some(2))
        );
        let bytes = STANDARD
            .decode(body["snapshot"]["imageBase64"].as_str().unwrap())
            .unwrap();
        assert!(bytes.len() <= 32 * 1024);
        assert_eq!(decode_png(&bytes), oracle::transmitted(rgba));
        self.digest = package::digest(&bytes);
        self.revision = revision;
        self.frames.push(bytes);
    }
}
pub fn verify_png(bytes: &[u8], expected: [u8; 4]) {
    assert_eq!(decode_png(bytes), expected.repeat(4));
}
fn decode_png(bytes: &[u8]) -> Vec<u8> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    assert_eq!((reader.info().width, reader.info().height), (2, 2));
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    pixels.truncate(info.buffer_size());
    match info.color_type {
        png::ColorType::Rgba => pixels,
        png::ColorType::Rgb => pixels
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => panic!("unexpected product PNG color type: {other:?}"),
    }
}

#[test]
fn formal_result_oracle_does_not_confuse_source_pixels_with_contained_transmission() {
    let color = [255, 0, 0, 255];
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&color.repeat(4)).unwrap();
        writer.finish().unwrap();
    }
    verify_png(&bytes, color);
    assert_ne!(decode_png(&bytes), oracle::transmitted(color));
}
