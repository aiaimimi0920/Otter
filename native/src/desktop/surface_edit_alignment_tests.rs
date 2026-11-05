//! 比较真实 EDIT WM_PRINTCLIENT 像素位置，而不只检查 GWL_STYLE。
use super::*;
fn ink_left(image: &[u8], empty: &[u8]) -> usize {
    image
        .chunks_exact(4)
        .zip(empty.chunks_exact(4))
        .enumerate()
        .filter_map(|(i, (a, b))| (a[..3] != b[..3]).then_some(i % 200))
        .min()
        .expect("text must render")
}
#[test]
fn surface_edit_alignment_moves_text_and_placeholder_without_changing_value() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    for multiline in [false, true] {
        let edit = Control::new(parent.dialog_owner().unwrap(), rect(), multiline, 100).unwrap();
        let hwnd = edit.hwnd().unwrap();
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
        let empty = pixels(hwnd);
        edit.install_placeholder("MMMM").unwrap();
        let mut hint_positions = Vec::new();
        let mut text_positions = Vec::new();
        for align in ["left", "center", "right"] {
            edit.set_alignment(Some(align)).unwrap();
            hint_positions.push(ink_left(&pixels(hwnd), &empty));
            assert_eq!(edit.text().unwrap(), "");
            edit.set_text("MMMM").unwrap();
            edit.select((1, 3)).unwrap();
            text_positions.push(ink_left(&pixels(hwnd), &empty));
            edit.set_alignment(Some(align)).unwrap();
            assert_eq!(edit.selection().unwrap(), (1, 3));
            assert_eq!(edit.text().unwrap(), "MMMM");
            edit.set_text("").unwrap();
            assert_eq!(
                unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } & !((ES_CENTER | ES_RIGHT) as isize),
                style & !((ES_CENTER | ES_RIGHT) as isize)
            );
        }
        assert!(
            hint_positions.windows(2).all(|p| p[0] < p[1]),
            "hint {hint_positions:?}"
        );
        assert!(
            text_positions.windows(2).all(|p| p[0] < p[1]),
            "text {text_positions:?}"
        );
        edit.set_alignment(Some("invalid")).unwrap();
        assert_eq!(ink_left(&pixels(hwnd), &empty), hint_positions[0]);
        edit.set_text("中文🙂").unwrap();
        edit.select((1, 3)).unwrap();
        edit.set_alignment(Some("right")).unwrap();
        assert_eq!(edit.text().unwrap(), "中文🙂");
        assert_eq!(edit.selection().unwrap(), (1, 3));
    }
}
