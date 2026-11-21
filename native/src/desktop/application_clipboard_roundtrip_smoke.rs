//! 仅在新建的非交互 window station 中验证真实系统剪贴板；普通 smoke 不访问剪贴板。
use super::*;
use crate::desktop::{clipboard_placement::Origin, copy_job, image::Image};
use windows::Win32::System::{DataExchange::*, StationsAndDesktops::*};

unsafe fn isolated() -> Result<bool> {
    let Some(expected) = std::env::var_os("OTTER_CLIPBOARD_TEST_STATION") else {
        return Ok(false);
    };
    let station = GetProcessWindowStation()?;
    let mut name = [0u16; 256];
    GetUserObjectInformationW(
        HANDLE(station.0),
        UOI_NAME,
        Some(name.as_mut_ptr().cast()),
        std::mem::size_of_val(&name) as u32,
        None,
    )?;
    let end = name.iter().position(|c| *c == 0).unwrap_or(name.len());
    let actual = String::from_utf16(&name[..end])
        .map_err(|_| Error::new(E_FAIL, "Invalid test window station name"))?;
    if actual.eq_ignore_ascii_case("WinSta0") || std::ffi::OsStr::new(&actual) != expected {
        return Err(Error::new(
            E_ACCESSDENIED,
            "Clipboard test requires an isolated window station",
        ));
    }
    Ok(true)
}

pub(in crate::desktop) unsafe fn run(app: &mut Application) -> Result<()> {
    if !isolated()? {
        return Err(Error::new(
            E_ACCESSDENIED,
            "Missing isolated clipboard test station",
        ));
    }
    let source = app.documents.len();
    app.add_prepared(
        vec![crate::desktop::open_image::PreparedDocument::from_image(
            Image {
                width: 8,
                height: 8,
                pixels: bytes(&[255; 256])?,
            },
            false,
        )?],
        false,
    )?;
    SetWindowPos(
        app.documents[source].window.0,
        None,
        600,
        600,
        480,
        240,
        SWP_NOACTIVATE | SWP_NOZORDER | SWP_SHOWWINDOW,
    )?;
    let doc = &app.documents[source];
    let origin = Origin::capture_at(doc.window.0, &doc.state, POINT { x: 630, y: 630 })
        .ok_or_else(|| Error::new(E_FAIL, "Cannot capture clipboard roundtrip source"))?;
    let base = origin.frame;
    let source_id = origin.unit.clone();
    // 注入光标采样并省略文件缓存计划；PNG/DIBV5 的 worker、系统发布和读取仍走产品路径。
    copy_job::start_guarded_at(
        doc.window.0,
        &doc.state,
        None,
        Some(GetClipboardSequenceNumber()),
        Some(origin),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while doc.state.copy_job.borrow().is_some() {
        if std::time::Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Clipboard publication worker timed out"));
        }
        copy_job::poll(doc.window.0, &doc.state);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    // 非交互 station 即使有 WS_VISIBLE 也不满足 IsWindowVisible；不伪造可见图内落点。
    // 此处保护真实读取到窗口交付的来源连续性；可见 A 的累计坐标由私有 desktop 回归保护。
    for (dx, dy) in [(1, 1), (80, 30), (470, 230), (3, 2), (220, 110), (20, 40)] {
        let doc = &app.documents[source];
        let snapshot = paste_input::read_image(doc.window.0, &doc.state, true)?;
        if !matches!(snapshot, paste_input::Snapshot::Editable(_)) {
            return Err(Error::new(
                E_FAIL,
                format!(
                    "Clipboard publication lost its source before paste: sequence={}",
                    GetClipboardSequenceNumber(),
                ),
            ));
        }
        let request = app
            .editable_clipboard
            .borrow()
            .position((base.x + dx, base.y + dy))
            .ok_or_else(|| Error::new(E_FAIL, "System clipboard read lost placement session"))?;
        if request.source() != (&source_id, base) {
            return Err(Error::new(
                E_FAIL,
                "Repeated system reads replaced the copied source",
            ));
        }
        paste_input::deliver_at(
            doc.window.0,
            &doc.state,
            snapshot,
            true,
            POINT {
                x: base.x + dx,
                y: base.y + dy,
            },
        )?;
        open_job::finish(doc.window.0, &doc.state, true);
        app.pending_windows(false)?;
        let pin = app.documents.last().unwrap();
        let mut rect = RECT::default();
        GetWindowRect(pin.window.0, &mut rect)?;
        if pin.state.unit_id == source_id
            || (rect.right - rect.left, rect.bottom - rect.top)
                != (base.width as i32, base.height as i32)
        {
            return Err(Error::new(
                E_FAIL,
                "System clipboard paste lost independent identity or copied dimensions",
            ));
        }
        SetWindowPos(
            pin.window.0,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )?;
    }
    println!("native-clipboard-roundtrip-smoke: publication=true system-read=true fixed-source=true new-windows=true copied-size=true count=6 clipboard-isolated=true");
    for pin in &app.documents[source..] {
        SendMessageW(pin.window.0, WM_CLOSE, None, None);
    }
    app.collect_closed();
    Ok(())
}
