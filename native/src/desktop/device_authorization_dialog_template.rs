//! 单一服务地址输入布局；无浏览器、外部字体或驻留渲染线程。
use windows::Win32::UI::WindowsAndMessaging::*;
pub(super) fn template(art: bool) -> Vec<u32> {
    fn string(data: &mut Vec<u16>, text: &str) {
        data.extend(text.encode_utf16());
        data.push(0);
    }
    fn dword(data: &mut Vec<u16>, value: u32) {
        data.extend([value as u16, (value >> 16) as u16]);
    }
    let mut data = Vec::new();
    dword(
        &mut data,
        WS_POPUP.0 | WS_CLIPCHILDREN.0 | DS_MODALFRAME as u32 | DS_SETFONT as u32,
    );
    dword(&mut data, 0);
    data.extend([8, 0, 0, 340, 162, 0, 0]);
    string(
        &mut data,
        if art {
            "打开 Art（原生预览）"
        } else {
            "连接 Loom"
        },
    );
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    for (id, class, style, x, y, width, height, text) in [
        (
            200,
            0x82,
            0,
            10,
            6,
            320,
            14,
            if art { "打开 Art" } else { "连接 Loom" },
        ),
        (202, 0x82, 0, 10, 52, 64, 14, "连接码（可选）"),
        (
            102,
            0x81,
            WS_BORDER.0 | WS_TABSTOP.0 | ES_PASSWORD as u32 | ES_AUTOHSCROLL as u32,
            78,
            50,
            252,
            18,
            "",
        ),
        (
            201,
            0x82,
            0,
            10,
            10,
            320,
            14,
            if art {
                "Art ID"
            } else {
                "服务地址（例如 https://loom.example.com:8443）"
            },
        ),
        (
            101,
            0x81,
            WS_BORDER.0 | WS_TABSTOP.0 | ES_AUTOHSCROLL as u32,
            10,
            28,
            320,
            18,
            "",
        ),
        (103, 0x82, 0, 10, 78, 320, 28, ""),
        (
            IDOK.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            210,
            120,
            55,
            18,
            if art { "确定" } else { "连接" },
        ),
        (
            IDCANCEL.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            275,
            120,
            55,
            18,
            "取消",
        ),
    ] {
        if data.len() % 2 != 0 {
            data.push(0);
        }
        dword(&mut data, WS_CHILD.0 | WS_VISIBLE.0 | style);
        dword(&mut data, 0);
        data.extend([
            x,
            if id == 200 { y } else { y + 14 },
            width,
            height,
            id,
            0xffff,
            class,
        ]);
        string(&mut data, text);
        data.push(0);
    }
    if data.len() % 2 != 0 {
        data.push(0);
    }
    data.chunks_exact(2)
        .map(|p| u32::from(p[0]) | (u32::from(p[1]) << 16))
        .collect()
}
