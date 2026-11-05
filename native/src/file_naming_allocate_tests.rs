use super::*;
#[test]
fn collision_suffix_keeps_unicode_limit_and_extension_contract() {
    let cancel = AtomicBool::new(false);
    let mut paths = Vec::new();
    let (_, path) = allocate_with(
        Path::new("output"),
        &"中🙂".repeat(100),
        Some(".P/N:G!!"),
        &cancel,
        |path| {
            paths.push(path.to_path_buf());
            if paths.len() < 3 {
                Err(io::ErrorKind::AlreadyExists.into())
            } else {
                Ok(())
            }
        },
    )
    .unwrap();
    assert_eq!(
        paths[0]
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .chars()
            .count(),
        120
    );
    assert!(paths[1].to_str().unwrap().ends_with("_2.png"));
    assert!(path.to_str().unwrap().ends_with("_3.png"));
    assert_eq!(
        path.file_stem().unwrap().to_str().unwrap().chars().count(),
        120
    );
    for (stem, expected) in [
        ("../CON", ".._CON.png"),
        ("CON", "_CON.png"),
        ("... ", "Otter.png"),
    ] {
        let (_, path) =
            allocate_with(Path::new("output"), stem, Some("!!"), &cancel, |_| Ok(())).unwrap();
        assert_eq!(path.file_name().unwrap(), expected);
        assert_eq!(path.parent(), Some(Path::new("output")));
    }
}

#[test]
fn exhaustion_errors_and_cancellation_are_bounded() {
    let cancel = AtomicBool::new(false);
    let mut count = 0;
    let result = allocate_with::<()>(Path::new("output"), "name", None, &cancel, |_| {
        count += 1;
        Err(io::ErrorKind::AlreadyExists.into())
    });
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(count, ATTEMPTS);
    count = 0;
    let result = allocate_with::<()>(Path::new("output"), "name", None, &cancel, |_| {
        count += 1;
        Err(io::ErrorKind::PermissionDenied.into())
    });
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(count, 1);
    let result = allocate_with::<()>(Path::new("output"), "name", None, &cancel, |_| {
        cancel.store(true, Ordering::Release);
        Err(io::ErrorKind::AlreadyExists.into())
    });
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
    assert!(allocate_with(
        Path::new("output"),
        "name",
        None,
        &cancel,
        |_| -> io::Result<()> { panic!("cancelled operation must not create") }
    )
    .is_err());
    assert!(allocate_with(
        Path::new("output"),
        &"x".repeat(super::super::MAX_CONTEXT_BYTES + 1),
        None,
        &cancel,
        |_| -> io::Result<()> { panic!("oversize operation must not create") }
    )
    .is_err());
}

#[cfg(windows)]
#[test]
fn concurrent_real_files_never_overwrite_and_exact_export_still_rejects_collision() {
    use std::io::Write;
    let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi");
    let path = root.join(format!("otter-unique-{:?}", unsafe {
        windows::Win32::System::Com::CoCreateGuid().unwrap()
    }));
    std::fs::create_dir(&path).unwrap();
    let handles: Vec<_> = (0..8)
        .map(|index| {
            let directory = path.clone();
            std::thread::spawn(move || {
                let (mut file, path) =
                    create_unique_file(&directory, "并发🙂", None, &AtomicBool::new(false))
                        .unwrap();
                write!(file, "writer-{index}").unwrap();
                (path, format!("writer-{index}"))
            })
        })
        .collect();
    let outputs: Vec<_> = handles
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    let distinct: std::collections::HashSet<_> = outputs.iter().map(|(path, _)| path).collect();
    assert_eq!(distinct.len(), 8);
    for (file, expected) in outputs {
        assert_eq!(std::fs::read_to_string(&file).unwrap(), expected);
        assert_eq!(
            create_exact(&file).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), expected);
        std::fs::remove_file(file).unwrap();
    }
    std::fs::remove_dir(path).unwrap();
}
