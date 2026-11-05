use super::*;
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root =
            Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-session-recovery-tests");
        std::fs::create_dir_all(root).unwrap();
        let path = root.join(format!("{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn target(&self) -> PathBuf {
        self.0.join("会话.json")
    }
    fn copy(&self, id: &str) -> PathBuf {
        let path = self.0.join(format!("会话.json.otter-recovery-{id}.json"));
        std::fs::write(&path, b"recovery fixture").unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // 本测试唯一 linshi 目录，只有直接创建的普通文件；无递归删除。
        for entry in std::fs::read_dir(&self.0).unwrap().flatten() {
            if entry.file_type().unwrap().is_file() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}
const A: &str = "01234567-89AB-CDEF-0123-456789ABCDEF";
const B: &str = "01234567-89AB-CDEF-0123-456789ABCDE0";
#[test]
fn missing_target_finds_only_exact_valid_recovery_name_and_never_modifies_it() {
    let fixture = Fixture::new();
    fixture.copy("not-a-guid");
    let path = fixture.copy(A);
    assert_eq!(
        find(&fixture.target(), &AtomicBool::new(false)).unwrap(),
        Some(path.clone())
    );
    assert!(!fixture.target().exists());
    assert_eq!(std::fs::read(path).unwrap(), b"recovery fixture");
}
#[test]
fn existing_target_wins_and_ambiguous_copies_are_rejected() {
    let fixture = Fixture::new();
    fixture.copy(A);
    fixture.copy(B);
    assert!(find(&fixture.target(), &AtomicBool::new(false)).is_err());
    std::fs::write(fixture.target(), b"current").unwrap();
    assert!(find(&fixture.target(), &AtomicBool::new(false))
        .unwrap()
        .is_none());
    assert_eq!(std::fs::read(fixture.target()).unwrap(), b"current");
}
#[test]
fn cancelled_search_does_not_scan_and_relative_paths_are_rejected() {
    let fixture = Fixture::new();
    assert_eq!(
        find(&fixture.target(), &AtomicBool::new(true))
            .unwrap_err()
            .kind(),
        io::ErrorKind::Interrupted
    );
    assert!(find(Path::new("relative.json"), &AtomicBool::new(false)).is_err());
}
