//! 可选的准备图片内容摘要；只在输入 worker 内计算，摘要不是来源授权或新鲜度凭证。
use super::*;
use sha2::{Digest, Sha256};
pub(super) fn capture(image: &Inline, cancel: &AtomicBool) -> Result<[u8; 32], Error> {
    let crate::loom_art_wire::Input::InlinePng {
        base64,
        width,
        height,
    } = image.descriptor()
    else {
        return Err(Error::InvalidInput);
    };
    let mut digest = Sha256::new();
    digest.update(width.to_le_bytes());
    digest.update(height.to_le_bytes());
    for chunk in base64.as_bytes().chunks(64 * 1024) {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        digest.update(chunk);
    }
    Ok(digest.finalize().into())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::shader_prepare::tests as staging;
    #[test]
    fn shader_fingerprint_is_opt_in_content_exact_dimension_sensitive_and_cancellable() {
        let editor = Editor::default();
        let image = |width, height, channel| {
            crate::desktop::loom_inline::prepare(
                width,
                height,
                vec![channel, 2, 3, 255, 4, 5, 6, 255].into(),
                &editor,
                &AtomicBool::new(false),
            )
            .unwrap()
        };
        let first = image(2, 1, 7);
        let hash = capture(&first, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            hash,
            capture(&image(2, 1, 7), &AtomicBool::new(false)).unwrap()
        );
        assert_ne!(
            hash,
            capture(&image(2, 1, 8), &AtomicBool::new(false)).unwrap()
        );
        assert_ne!(
            hash,
            capture(&image(1, 2, 7), &AtomicBool::new(false)).unwrap()
        );
        assert!(matches!(
            capture(&first, &AtomicBool::new(true)),
            Err(Error::Cancelled)
        ));
        let source = |port: &str| {
            Source::capture(
                port.into(),
                staging::id(port),
                1,
                1,
                vec![1, 2, 3, 255].into(),
                &editor,
            )
            .unwrap()
        };
        let mut marked = source("marked");
        marked.fingerprint_for("unrelated");
        assert!(!marked.fingerprint);
        marked.fingerprint_for("marked");
        let batch = prepare(
            staging::id("target"),
            editor.document_stamp(),
            vec![source("ordinary"), marked],
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(batch.fingerprint("ordinary"), None);
        assert!(batch.fingerprint("marked").is_some());
        assert_eq!(batch.fingerprint("missing"), None);
    }
}
