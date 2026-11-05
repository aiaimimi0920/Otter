//! 系统 UIA 客户端读取真实已绘制图片；alt、装饰过滤、叶节点与旧 provider 撤销。
use super::*;
use crate::{
    image_decode::Image, surface_image_paint::Bitmap, surface_presentation::Images,
    surface_scene::Scene,
};
fn scene(label: &str) -> Scene {
    Scene::parse(&serde_json::to_vec(&serde_json::json!({"id":"tree","type":"column","children":[
        {"id":"named","type":"image","props":{"alt":label,"disabled":true},"accessibility":{"label":"Wrong"},"layout":{"width":32,"height":32},"events":{"click":"ignored"},"children":[{"id":"ignored","type":"input"},{"id":"nested-image","type":"image"}]},
        {"id":"empty","type":"image","props":{"alt":""},"layout":{"width":32,"height":32}},
        {"id":"missing","type":"image","accessibility":{"label":"Wrong"},"layout":{"width":32,"height":32}},
        {"id":"number","type":"image","props":{"alt":42},"layout":{"width":32,"height":32}}
    ]})).unwrap()).unwrap()
}
fn images() -> Images {
    let bitmap = Bitmap::new(Image {
        width: 1,
        height: 1,
        pixels: vec![255; 4].into(),
    })
    .unwrap();
    ["named", "empty", "missing", "number"]
        .map(|id| (id.to_owned(), bitmap.clone()))
        .into()
}
#[test]
fn uia_image_unavailable_inline_keeps_alt_without_invoke_or_child_control() {
    let _com = super::super::super::super::Apartment::new().unwrap();
    let scene = Scene::parse("{\"id\":\"tree\",\"type\":\"column\",\"children\":[{\"id\":\"named\",\"type\":\"image\",\"props\":{\"src\":\"data:image/png,bad\",\"alt\":\"缺失图片\"}}]}".as_bytes()).unwrap().with_image_availability(|_| false);
    let window = super::super::super::SurfaceWindow::new(scene, 200, 180).unwrap();
    query(window.hwnd.0 as usize, |hwnd| unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let _com = Apartment;
        let client = client()?;
        let walker = client.RawViewWalker()?;
        let image = walker.GetFirstChildElement(&tree(&client, hwnd)?)?;
        assert_eq!(image.CurrentName()?.to_string(), "缺失图片");
        assert_eq!(image.CurrentControlType()?, UIA_ImageControlTypeId);
        assert!(!image.CurrentIsKeyboardFocusable()?.as_bool());
        assert!(image.GetCurrentPattern(UIA_InvokePatternId).is_err());
        assert!(walker.GetFirstChildElement(&image).is_err());
        Ok(())
    });
    assert!(window.take_error().is_none());
    window.close().unwrap();
}
unsafe fn client() -> Result<IUIAutomation2> {
    let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
    client.SetConnectionTimeout(3000)?;
    client.SetTransactionTimeout(3000)?;
    Ok(client)
}
unsafe fn tree(client: &IUIAutomation2, hwnd: usize) -> Result<IUIAutomationElement> {
    let window = client.ElementFromHandle(HWND(hwnd as *mut _))?;
    // HWND raw children 还可能有系统标题栏；按声明场景 ID 定位，不能假定第一个是内容。
    let condition = client.CreatePropertyCondition(
        UIA_AutomationIdPropertyId,
        &windows::Win32::System::Variant::VARIANT::from("tree"),
    )?;
    window.FindFirst(TreeScope_Descendants, &condition)
}
#[test]
fn uia_image_system_client_reads_alt_filters_decoration_and_ignores_children() {
    let _com = super::super::super::super::Apartment::new().unwrap();
    let window =
        super::super::super::SurfaceWindow::with_images(scene("地图"), 200, 180, images()).unwrap();
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    query(window.hwnd.0 as usize, |hwnd| {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        }
        let _com = Apartment;
        unsafe {
            let client = client()?;
            let tree = tree(&client, hwnd)?;
            let walker = client.RawViewWalker()?;
            let named = walker.GetFirstChildElement(&tree)?;
            assert_eq!(named.CurrentName()?.to_string(), "地图");
            assert_eq!(named.CurrentControlType()?, UIA_ImageControlTypeId);
            assert!(named.CurrentIsControlElement()?.as_bool());
            assert!(named.CurrentIsContentElement()?.as_bool());
            assert!(named.CurrentIsEnabled()?.as_bool());
            assert!(!named.CurrentIsKeyboardFocusable()?.as_bool());
            assert!(named.GetCurrentPattern(UIA_InvokePatternId).is_err());
            assert!(walker.GetFirstChildElement(&named).is_err());
            let mut next = named.clone();
            for id in ["empty", "missing", "number"] {
                next = walker.GetNextSiblingElement(&next)?;
                assert_eq!(next.CurrentAutomationId()?.to_string(), id);
                assert!(next.CurrentName()?.is_empty());
                assert!(!next.CurrentIsControlElement()?.as_bool());
                assert!(!next.CurrentIsContentElement()?.as_bool());
            }
            assert!(walker.GetNextSiblingElement(&next).is_err());
            let control = client.ControlViewWalker()?;
            let content = client.ContentViewWalker()?;
            assert!(control.GetNextSiblingElement(&named).is_err());
            assert!(content.GetNextSiblingElement(&named).is_err());
            Ok(())
        }
    });
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
    window.close().unwrap();
}
#[test]
fn uia_image_system_client_replacement_revokes_old_provider() {
    let _com = super::super::super::super::Apartment::new().unwrap();
    let window =
        super::super::super::SurfaceWindow::with_images(scene("Old"), 200, 180, images()).unwrap();
    let (held, ready) = std::sync::mpsc::channel();
    let (changed, resume) = std::sync::mpsc::channel();
    query_with(
        window.hwnd.0 as usize,
        move |hwnd| {
            unsafe {
                CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            }
            let _com = Apartment;
            unsafe {
                let client = client()?;
                let old = client
                    .RawViewWalker()?
                    .GetFirstChildElement(&tree(&client, hwnd)?)?;
                assert_eq!(old.CurrentName()?.to_string(), "Old");
                held.send(()).unwrap();
                resume.recv_timeout(Duration::from_secs(5)).unwrap();
                assert_eq!(
                    old.CurrentName().unwrap_err().code(),
                    windows::core::HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32)
                );
                let new = client
                    .RawViewWalker()?
                    .GetFirstChildElement(&tree(&client, hwnd)?)?;
                assert_eq!(new.CurrentName()?.to_string(), "New");
                Ok(())
            }
        },
        || {
            if ready.try_recv().is_ok() {
                window.replace_with_images(scene("New"), images()).unwrap();
                changed.send(()).unwrap();
            }
        },
    );
    assert!(window.take_error().is_none());
    window.close().unwrap();
}
