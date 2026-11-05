//! 真实Loom联合专属隔离输入/有界文件屏障；不读取用户身份，不向用户窗口注入输入。
use super::*;
use serde_json::{json, Value};
use std::path::PathBuf;
pub(super) struct Fixture {
    pub root: PathBuf,
    pub origin: String,
    pub profile: PathBuf,
    pub deadline: Instant,
}
impl Fixture {
    pub fn new() -> Self {
        let input = PathBuf::from(std::env::var_os("OTTER_TILE_JOINT_INPUT").unwrap());
        assert!(input.is_absolute() && input.is_file());
        let value: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
        let origin = value["origin"].as_str().unwrap().to_owned();
        let url = url::Url::parse(&origin).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("127.0.0.1"));
        let root = input.parent().unwrap().to_owned();
        Self {
            profile: root.join("native-profile"),
            root,
            origin,
            deadline: Instant::now() + Duration::from_secs(30),
        }
    }
    pub fn mark(&self, name: &str, value: Value) {
        let pending = self.root.join(format!("{name}.pending"));
        std::fs::write(&pending, serde_json::to_vec(&value).unwrap()).unwrap();
        std::fs::rename(pending, self.root.join(name)).unwrap();
    }
    pub fn pump(&self) {
        assert!(
            Instant::now() < self.deadline,
            "Tile child deadline: {}",
            self.root.display()
        );
        pump().unwrap();
        wait(10);
    }
    pub fn phase(&self, name: &str) {
        while !self.root.join(name).exists() {
            self.pump();
        }
    }
    pub fn menu(&self, window: &Window, owner: &mut Owner, action: usize) -> Event {
        unsafe {
            SendMessageW(
                window.hwnd,
                WM_COMMAND,
                Some(WPARAM(action)),
                Some(LPARAM(0)),
            );
        }
        let action = window.take_action().unwrap();
        assert!(
            authorization_action(window, owner, action, &self.origin, Some(&self.profile)).unwrap()
        );
        loop {
            for event in owner.poll() {
                match event {
                    Event::Pairing { .. } | Event::Authorized { .. } => return event,
                    Event::Failed { error, .. } => panic!("Tile menu authorization: {error:?}"),
                    _ => {}
                }
            }
            self.pump();
        }
    }
}
pub(super) struct Apartment;
impl Apartment {
    pub fn new() -> Self {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        }
        Self
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
pub(super) fn mark_pixels(f: &Fixture, runtime: &Runtime, name: &str) {
    let s = runtime.shared.lock();
    let frame = s.frame.as_ref().unwrap();
    let image = frame.images.values().next().unwrap();
    // 手工RGBA红/绿/蓝/白的PBGRA oracle，不从生产转换或绘制代码计算预期。
    assert_eq!((image.width, image.height), (2, 2));
    assert_eq!(
        &*image.pixels,
        &[0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 255, 255, 255, 255]
    );
    assert_eq!(frame.images.len(), 1);
    assert_eq!(s.presented.as_ref().unwrap().0, frame.key);
    f.mark(name, json!({"revision":frame.key.revision,"sourcePbgra":[0,0,255,255,0,255,0,255,255,0,0,255,255,255,255,255],"nativePresentConfirmed":true}));
}
