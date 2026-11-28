//! Use the real WIC encoder and staged file writer against a fresh isolated directory.
use super::*;
use std::fs;
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let dir = crate::test_temp_root().join(format!("otter-archive-{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }
    fn files(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("archive-failure-evidence: {}", self.0.display());
            return;
        }
        for path in self.files() {
            fs::remove_file(path).unwrap();
        }
        fs::remove_dir(&self.0).unwrap();
    }
}
fn sample() -> Image {
    Image {
        width: 3,
        height: 2,
        pixels: [12, 34, 56, 255].repeat(6).into(),
    }
}
fn settle(s: &State) {
    let mut a = s.image_archive.borrow_mut();
    a.pump(true);
    assert!(a.error.is_none(), "{:?}", a.error);
    assert!(!a.busy());
}
#[test]
fn archive_requires_consent_and_distinguishes_repeated_copy_from_style_change() {
    let dir = Directory::new();
    let im = sample();
    let state = State::new(Some(im.clone()), false).unwrap();
    state.image_archive.borrow_mut().target = Some(grant(&dir.0).unwrap());
    let png = super::super::export::png_raw(&im, &AtomicBool::new(false)).unwrap();
    let copy = || {
        copied(
            &state,
            Stamp::capture(&state.canvas.borrow(), None),
            png.clone(),
        )
    };
    pinned(&state);
    copy();
    settle(&state);
    assert!(dir.files().is_empty());
    state.image_archive.borrow_mut().copy = true;
    copy();
    copy();
    settle(&state);
    copy();
    settle(&state);
    assert_eq!(dir.files().len(), 1);
    assert_eq!(fs::read(&dir.files()[0]).unwrap(), png.as_ref());
    state.canvas.borrow_mut().normal_opacity = 0.5;
    copy();
    settle(&state);
    assert_eq!(dir.files().len(), 2);
    state.image_archive.borrow_mut().pin = true;
    state.canvas.borrow_mut().normal_opacity = 0.75;
    pinned(&state);
    settle(&state);
    assert_eq!(dir.files().len(), 3);
    assert!(dir.files().iter().all(|p| p.extension().unwrap() == "png"));
}
#[test]
fn archive_frozen_crops_only_the_selected_pixels_and_reaps_admitted_writes() {
    let _apartment = super::super::Apartment::new().unwrap();
    let dir = Directory::new();
    let state = State::new(None, false).unwrap();
    {
        let mut a = state.image_archive.borrow_mut();
        a.copy = true;
        a.target = Some(grant(&dir.0).unwrap());
    }
    assert!(!frozen_enabled());
    let scope = Selector::enter(&state.image_archive);
    let image = sample();
    let rect = RECT {
        left: 1,
        top: 0,
        right: 2,
        bottom: 1,
    };
    let origin = POINT { x: -200, y: 10 };
    frozen(&image, rect, origin);
    frozen(&image, rect, origin);
    drop(scope);
    assert!(!frozen_enabled());
    settle(&state);
    let files = dir.files();
    assert_eq!(files.len(), 1);
    let pixel = super::super::export::decode_png(fs::read(&files[0]).unwrap());
    assert_eq!((pixel.width, pixel.height), (1, 1));
    assert_eq!(&*pixel.pixels, &[12, 34, 56, 255]);
}

#[test]
fn failed_write_is_reported_once_and_does_not_poison_retry_or_replace_a_file() {
    use std::os::windows::fs::OpenOptionsExt;
    use windows::Win32::Storage::FileSystem::{FILE_ADD_FILE, FILE_FLAG_BACKUP_SEMANTICS};
    let dir = Directory::new();
    let occupied = dir.0.join("Otter_pin.png");
    fs::write(&occupied, b"keep").unwrap();
    let state = State::new(Some(sample()), false).unwrap();
    {
        let mut a = state.image_archive.borrow_mut();
        a.pin = true;
        assert!(grant(&occupied).is_err(), "a file is not a directory grant");
        a.target = Some(grant(&dir.0).unwrap());
    }
    // A conflicting directory writer prevents a safe staging open. Retry is
    // possible after it closes, without granting a different destination.
    let competing = fs::OpenOptions::new()
        .access_mode(FILE_ADD_FILE.0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
        .open(&dir.0)
        .unwrap();
    pinned(&state);
    let mut a = state.image_archive.borrow_mut();
    a.pump(true);
    assert!(a.error.take().is_some());
    assert!(!a.busy());
    a.pump(false);
    assert!(a.error.is_none());
    assert_eq!(fs::read(&occupied).unwrap(), b"keep");
    drop(competing);
    drop(a);
    pinned(&state);
    settle(&state);
    assert_eq!(dir.files().len(), 2);
    assert_eq!(fs::read(&occupied).unwrap(), b"keep");
    assert!(dir.0.join("Otter_pin_2.png").is_file());
}

#[test]
fn consent_pins_only_the_displayed_directory_and_admitted_jobs_keep_their_grant() {
    let profile = Directory::new();
    let second = Directory::new();
    let state = State::new(Some(sample()), false).unwrap();
    let saved = profile.0.join("saved");
    {
        let mut a = state.image_archive.borrow_mut();
        assert!(
            a.directory().is_err(),
            "no ambient default without a profile"
        );
        a.bind_profile(Some(&profile.0));
        assert!(!a.copy && !a.pin);
        assert!(a.target.is_none());
        assert!(
            !saved.exists(),
            "profile binding must not create an archive"
        );
        assert_eq!(a.directory().unwrap(), saved);
        assert!(
            grant(&saved).is_err(),
            "picker must not recreate a missing selection"
        );
        assert!(a.confirm(&second.0).is_err());
        assert!(!saved.exists(), "opening a menu did not authorize creation");
        a.confirm(&saved).unwrap();
        a.pin = true;
    }
    pinned(&state);
    state.image_archive.borrow_mut().target = Some(grant(&second.0).unwrap());
    pinned(&state); // Same pin, new destination, both writes must finish.
    settle(&state);
    let old: Vec<_> = fs::read_dir(&saved)
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect();
    assert_eq!(old.len(), 1);
    assert_eq!(second.files().len(), 1);
    assert_eq!(
        fs::read(&old[0]).unwrap(),
        fs::read(&second.files()[0]).unwrap()
    );
    drop(state);
    fs::remove_file(&old[0]).unwrap();
    fs::remove_dir(saved).unwrap();
}

#[test]
fn binding_a_profile_keeps_an_explicit_archive_grant_and_flags() {
    let profile = Directory::new();
    let selected = Directory::new();
    let mut archive = Owner::default();
    archive.target = Some(grant(&selected.0).unwrap());
    archive.copy = true;
    archive.bind_profile(Some(&profile.0));
    assert!(archive.copy);
    assert!(!archive.pin);
    assert_eq!(archive.directory().unwrap(), selected.0);
    assert!(!profile.0.join("saved").exists());
}

#[test]
fn substituted_directory_is_refused_and_cancelled_staging_retires_only_its_handle() {
    let confirmed = Directory::new();
    let substitute = Directory::new();
    let mut grant = grant(&confirmed.0).unwrap();
    // Keep the real OS handle alive but substitute the lookup result. The
    // writer must compare identities before opening even a temporary file.
    Arc::get_mut(&mut grant).unwrap().path = substitute.0.clone();
    assert!(grant.begin_file().is_err());
    assert!(substitute.files().is_empty());
    Arc::get_mut(&mut grant).unwrap().path = confirmed.0.clone();
    let keep = confirmed.0.join("existing.png");
    fs::write(&keep, b"unrelated").unwrap();
    let stage = grant.begin_file().unwrap();
    assert_eq!(confirmed.files().len(), 2);
    let err = stage
        .publish(b"not published", "cancelled", &AtomicBool::new(true))
        .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Interrupted);
    assert_eq!(confirmed.files(), [keep.clone()]);
    assert_eq!(fs::read(keep).unwrap(), b"unrelated");
}
