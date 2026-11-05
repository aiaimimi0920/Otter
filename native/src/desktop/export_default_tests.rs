use super::*;
use std::fs;
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(
            PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
                .join(format!("otter-default-export-{:?}", unsafe {
                    windows::Win32::System::Com::CoCreateGuid().unwrap()
                })),
        )
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let saved = self.0.join("saved");
        if saved.is_dir() {
            for entry in fs::read_dir(&saved).unwrap() {
                fs::remove_file(entry.unwrap().path()).unwrap();
            }
            fs::remove_dir(saved).unwrap();
        } else if saved.is_file() {
            fs::remove_file(saved).unwrap();
        }
        if self.0.exists() {
            fs::remove_dir(&self.0).unwrap();
        }
    }
}
fn state() -> State {
    let state = State::new(
        Some(crate::desktop::image::Image {
            width: 2,
            height: 2,
            pixels: vec![255; 16].into(),
        }),
        false,
    )
    .unwrap();
    state.naming.borrow_mut().sticker_save_pattern = "固定_{width}x{height}".into();
    state
}
#[test]
fn default_save_creates_png_on_worker_increments_and_keeps_frozen_settings() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let directory = Directory::new();
    let state = state();
    let first = prepare(&state, directory.0.clone()).unwrap().unwrap();
    assert!(!directory.0.exists());
    state.naming.borrow_mut().sticker_save_pattern = "后来的名字".into();
    let path = std::thread::spawn(move || first.save())
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(path.file_name().unwrap(), "固定_2x2.png");
    let original = fs::read(&path).unwrap();
    let image = export::decode_png(original.clone());
    assert_eq!((image.width, image.height), (2, 2));
    assert_eq!(&*image.pixels, &[255; 16]);
    state.naming.borrow_mut().sticker_save_pattern = "固定_{width}x{height}".into();
    let second = prepare(&state, directory.0.clone()).unwrap().unwrap();
    let next = std::thread::spawn(move || second.save())
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(next.file_name().unwrap(), "固定_2x2_2.png");
    assert_eq!(fs::read(path).unwrap(), original);
}
#[test]
fn missing_image_and_bad_name_do_not_create_directories() {
    let directory = Directory::new();
    let empty = State::new(None, false).unwrap();
    assert!(prepare(&empty, directory.0.clone()).unwrap().is_none());
    let state = state();
    state.naming.borrow_mut().sticker_save_pattern = "{bad}".into();
    assert!(prepare(&state, directory.0.clone()).is_err());
    assert!(!directory.0.exists());
}
#[test]
fn existing_non_directory_is_preserved_and_command_is_allowed_when_group_locked() {
    let directory = Directory::new();
    fs::create_dir(&directory.0).unwrap();
    let blocker = directory.0.join("saved");
    fs::write(&blocker, b"keep").unwrap();
    let prepared = prepare(&state(), directory.0.clone()).unwrap().unwrap();
    assert!(std::thread::spawn(move || prepared.save())
        .join()
        .unwrap()
        .is_err());
    assert_eq!(fs::read(blocker).unwrap(), b"keep");
    assert!(crate::desktop::application::handles(
        crate::desktop::application::SAVE_PNG_DEFAULT
    ));
    assert!(crate::desktop::group_lock::allows(
        crate::desktop::application::SAVE_PNG_DEFAULT
    ));
}
