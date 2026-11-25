//! Native OCR cancellation proof against the already-warmed, isolated real plugin.
use super::*;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{FILETIME, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{
            GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, WaitForSingleObject,
            PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        },
    },
};

struct Process(OwnedHandle);

impl Process {
    unsafe fn open(pid: u32, expected: &std::path::Path) -> Self {
        let raw = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            false,
            pid,
        )
        .unwrap();
        let process = Self(OwnedHandle::from_raw_handle(raw.0));
        let mut path = vec![0u16; 32768];
        let mut length = path.len() as u32;
        QueryFullProcessImageNameW(
            process.handle(),
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &mut length,
        )
        .unwrap();
        use std::os::windows::ffi::OsStringExt;
        let actual = PathBuf::from(std::ffi::OsString::from_wide(&path[..length as usize]));
        assert!(expected.is_absolute());
        assert_eq!(
            actual.canonicalize().unwrap(),
            expected.canonicalize().unwrap()
        );
        process
    }

    fn handle(&self) -> HANDLE {
        HANDLE(self.0.as_raw_handle())
    }

    unsafe fn cpu_100ns(&self) -> u64 {
        let [mut created, mut exited, mut kernel, mut user] = [FILETIME::default(); 4];
        GetProcessTimes(
            self.handle(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
        .unwrap();
        let ticks =
            |time: FILETIME| (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
        ticks(kernel) + ticks(user)
    }
}

pub(super) unsafe fn verify(
    input: &Value,
    directory: Arc<Snapshot>,
    identity: crate::loom_bridge_identity::Identity,
    small_image: &std::path::Path,
    expected: &str,
) {
    let pid = u32::try_from(input["pid"].as_u64().unwrap()).unwrap();
    let host = PathBuf::from(input["host"].as_str().unwrap());
    let heavy_image = PathBuf::from(input["image"].as_str().unwrap());
    assert!(heavy_image.is_absolute() && heavy_image != small_image);
    let process = Process::open(pid, &host);
    let mut fixture = Fixture::with_image(directory.clone(), || {
        crate::image_decode::Image::open(&heavy_image).unwrap()
    });
    let original = fixture.state().extension_document.borrow().store.stamp();
    let copies = super::super::super::super::clipboard::capture_extension_copy(|| {
        let before = process.cpu_100ns();
        fixture.begin_button(false, identity.clone());
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            let pending = fixture
                .owner
                .pending
                .as_ref()
                .expect("OCR ended before cancel");
            assert!(
                !pending.job.apply_pending(),
                "OCR result arrived before cancel"
            );
            assert_eq!(WaitForSingleObject(process.handle(), 0), WAIT_TIMEOUT);
            let delta = process.cpu_100ns().saturating_sub(before);
            if delta >= 1_000_000 {
                // Count only the warmed plugin's process CPU, not PNG upload or daemon CPU.
                println!("ocr_inflight pid={pid} cpu_100ns={delta} apply_pending=false");
                break;
            }
            fixture.tick();
            assert!(Instant::now() < deadline, "warmed OCR did not start");
            thread::sleep(Duration::from_millis(2));
        }
        let cancelled = fixture
            .owner
            .pending
            .as_ref()
            .unwrap()
            .intent
            .cancel
            .clone();
        let started = Instant::now();
        fixture.click_button(false);
        assert!(cancelled.load(Ordering::Acquire));
        fixture.finish();
        // Wait on the original process handle so a reused PID cannot pass this check.
        assert_eq!(WaitForSingleObject(process.handle(), 3000), WAIT_OBJECT_0);
        assert!(started.elapsed() < Duration::from_secs(3));
        println!(
            "ocr_cancelled pid={pid} exit_ms={}",
            started.elapsed().as_millis()
        );
    });
    assert!(copies.is_empty());
    assert!(original.matches(&fixture.state().extension_document.borrow().store));
    drop(fixture);

    let mut retry = Fixture::with_image(directory, || {
        crate::image_decode::Image::open(small_image).unwrap()
    });
    let copies = super::super::super::super::clipboard::capture_extension_copy(|| {
        retry.begin_button(false, identity);
        retry.finish();
    });
    let clipboard = format!("{}\0", expected.replace('\n', "\r\n"));
    assert_eq!(copies, [clipboard.encode_utf16().collect::<Vec<_>>()]);
    println!("ocr_cancel_retry_exact=true");
}
