//! 有界批次由同一个工作线程顺序准备；全部成功才发布，不持有半批可见窗口。
use super::*;

pub(super) fn load_inputs(
    input: Input,
    pinned: bool,
    cancel: &AtomicBool,
) -> std::result::Result<Option<Vec<PreparedDocument>>, String> {
    let inputs = match input {
        Input::Files(paths) => {
            if paths.is_empty() || paths.len() > MAX_BATCH {
                return Err("Image import batch exceeds limit".into());
            }
            paths.into_iter().map(Input::File).collect::<Vec<_>>()
        }
        input => vec![input],
    };
    let mut prepared = Vec::new();
    prepared
        .try_reserve_exact(inputs.len())
        .map_err(|_| "Cannot allocate import batch")?;
    for input in inputs {
        let Some(document) = load_input(input, pinned, cancel)? else {
            return Ok(None);
        };
        prepared.push(document);
    }
    if cancel.load(Ordering::Acquire) {
        Ok(None)
    } else {
        Ok(Some(prepared))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn image_path() -> PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("src-tauri/icons/32x32.png")
    }
    #[test]
    fn batch_decodes_all_in_order_and_requires_bounded_nonempty_input() {
        let paths = vec![image_path(), image_path()];
        let documents = load_inputs(Input::Files(paths), true, &AtomicBool::new(false))
            .unwrap()
            .unwrap();
        assert_eq!(documents.len(), 2);
        assert!(documents.iter().all(|document| document.image.width == 32));
        for paths in [Vec::new(), vec![image_path(); MAX_BATCH + 1]] {
            assert!(load_inputs(Input::Files(paths), true, &AtomicBool::new(false)).is_err());
        }
    }
    #[test]
    fn batch_cancellation_and_late_decode_failure_publish_no_partial_result() {
        let paths = vec![image_path(), image_path()];
        assert!(
            load_inputs(Input::Files(paths), true, &AtomicBool::new(true))
                .unwrap()
                .is_none()
        );
        let invalid = image_path().with_file_name("otter-missing-batch-fixture.png");
        assert!(load_inputs(
            Input::Files(vec![image_path(), invalid]),
            true,
            &AtomicBool::new(false)
        )
        .is_err());
    }
}
