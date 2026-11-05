use super::*;
use std::{
    fs,
    time::{Duration, Instant},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(
            PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
                .join(format!("otter-drag-prepare-{:?}", unsafe {
                    windows::Win32::System::Com::CoCreateGuid().unwrap()
                })),
        )
    }
    fn input(&self) -> Input {
        let pixels: crate::pixels::Pixels = vec![255; 64].into();
        let editor = crate::editor::Editor::with_layers(crate::erase_layers::Layers {
            size: (4, 4),
            base: pixels.clone(),
            annotation_size: None,
            annotations: None,
        })
        .unwrap();
        Input {
            width: 4,
            height: 4,
            pixels,
            editor,
            opacity: 0.5,
            root: self.0.clone(),
            stem: "拖出 🦦".into(),
            policy: Default::default(),
        }
    }
    fn entries(&self) -> usize {
        fs::read_dir(self.0.join("clipboard_cache"))
            .unwrap()
            .count()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let cache = self.0.join("clipboard_cache");
        if cache.exists() {
            for entry in fs::read_dir(&cache).unwrap() {
                fs::remove_file(entry.unwrap().path()).unwrap();
            }
            fs::remove_dir(cache).unwrap();
        }
        if self.0.exists() {
            fs::remove_dir(&self.0).unwrap();
        }
    }
}
fn wait(job: &Job) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !job.is_finished() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn worker_encodes_committed_crop_opacity_and_publishes_only_on_explicit_commit() {
    let fixture = Fixture::new();
    let _apartment = super::super::Apartment::new().unwrap();
    let mut input = fixture.input();
    input
        .editor
        .crop_to(crate::crop_geometry::Rect {
            x: 1.0,
            y: 1.0,
            width: 2.0,
            height: 2.0,
        })
        .unwrap();
    let mut job = Job::start(input).unwrap();
    wait(&job);
    let staged = job.take_ready().unwrap().unwrap().unwrap();
    assert_eq!((staged.preview.width, staged.preview.height), (2, 2));
    assert!(staged
        .preview
        .pixels
        .chunks_exact(4)
        .all(|pixel| (127..=128).contains(&pixel[3])));
    assert!(job.take_ready().is_none());
    assert_eq!(fixture.entries(), 2); // lock + 独占暂存，尚无 PNG/归属记录。
    let path = staged.commit(&AtomicBool::new(false)).unwrap();
    assert_eq!(path.file_name().unwrap(), "拖出 🦦.png");
    let image = super::super::image::Image::open(&path).unwrap();
    assert_eq!((image.width, image.height), (2, 2));
    assert!(image
        .pixels
        .chunks_exact(4)
        .all(|pixel| (127..=128).contains(&pixel[3])));
    drop(job);
    assert!(path.exists());
    let mut next = Job::start(fixture.input()).unwrap();
    wait(&next);
    let next_path = next
        .take_ready()
        .unwrap()
        .unwrap()
        .unwrap()
        .commit(&AtomicBool::new(false))
        .unwrap();
    assert_eq!(next_path.file_name().unwrap(), "拖出 🦦_2.png");
    assert!(path.exists());
}

#[test]
fn cancellation_and_owner_drop_discard_completed_staging_and_release_lock() {
    let fixture = Fixture::new();
    let mut job = Job::start(fixture.input()).unwrap();
    wait(&job);
    assert_eq!(fixture.entries(), 2);
    job.cancel();
    assert!(job.take_ready().unwrap().unwrap().is_none());
    assert_eq!(fixture.entries(), 1);
    let next = Job::start(fixture.input()).unwrap();
    wait(&next);
    assert_eq!(fixture.entries(), 2);
    drop(next);
    assert_eq!(fixture.entries(), 1);
}

#[test]
fn invalid_dimensions_opacity_and_policy_fail_without_starting_worker_or_touching_disk() {
    let fixture = Fixture::new();
    let mut input = fixture.input();
    input.width = 0;
    assert!(Job::start(input).is_err());
    let mut input = fixture.input();
    input
        .editor
        .begin(
            crate::editor::Tool::Brush,
            crate::viewport::Point { x: 1.0, y: 1.0 },
        )
        .unwrap();
    assert!(Job::start(input).is_err());
    for opacity in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
        let mut input = fixture.input();
        input.opacity = opacity;
        assert!(Job::start(input).is_err());
    }
    let mut input = fixture.input();
    input.policy.temp_cache_retention_days = 3651;
    assert!(Job::start(input).is_err());
    assert!(!fixture.0.exists());
}

#[test]
fn pre_cancelled_prepare_does_not_create_cache_and_invalid_root_error_is_observable() {
    let fixture = Fixture::new();
    assert!(prepare(fixture.input(), &AtomicBool::new(true))
        .unwrap()
        .is_none());
    assert!(!fixture.0.exists());
    fs::write(&fixture.0, b"not a directory").unwrap();
    let mut job = Job::start(fixture.input()).unwrap();
    wait(&job);
    assert!(job.take_ready().unwrap().is_err());
    assert_eq!(fs::read(&fixture.0).unwrap(), b"not a directory");
    fs::remove_file(&fixture.0).unwrap();
}
