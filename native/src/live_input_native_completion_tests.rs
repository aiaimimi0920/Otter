//! 生产 Publication/源 hook/Modal 完成观察；自有控件，模拟回调到达，不运行产品或 WGC。
use super::*;
use crate::live_window_target::recovery::popup_layer::fixture::Fixture;
use std::cell::Cell;
use windows::core::w;

#[derive(Clone, Copy)]
enum Effect {
    RevokeCycle,
    HideOwner,
    DestroyDestination,
}
thread_local! { static EFFECT: Cell<Option<Effect>> = const { Cell::new(None) }; }
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_MOUSEMOVE {
        let request = otter_live_hover_hook::native_dispatch::collecting().unwrap();
        match EFFECT.with(Cell::get).unwrap() {
            Effect::RevokeCycle => {
                SetPropW(request.owner, protocol::EPOCH, Some(HANDLE::default())).unwrap()
            }
            Effect::HideOwner => {
                let _ = ShowWindow(request.owner, SW_HIDE);
            }
            Effect::DestroyDestination => DestroyWindow(hwnd).unwrap(),
        }
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
struct Child(HWND);
impl Child {
    fn new(root: HWND) -> Self {
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_CHILD,
                0,
                0,
                40,
                30,
                Some(root),
                None,
                None,
                None,
            )
        }
        .unwrap();
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
        }
        Self(hwnd)
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        EFFECT.with(|slot| slot.set(None));
        unsafe {
            if IsWindow(Some(self.0)).as_bool() {
                DestroyWindow(self.0).unwrap();
            }
        }
    }
}

#[test]
fn returned_native_callback_is_not_rejected_when_the_operation_changes_source_or_control() {
    for (index, effect) in [
        Effect::RevokeCycle,
        Effect::HideOwner,
        Effect::DestroyDestination,
    ]
    .into_iter()
    .enumerate()
    {
        let fixture = Fixture::new();
        let child = Child::new(fixture.root.0);
        let root = Target::bind(fixture.root.0).unwrap();
        let target = Destination::bind(&root, child.0).unwrap();
        let control = fixture.owner.identity().capture_control().unwrap();
        let nonce = 211 + index;
        let publication = Publication::publish(
            control,
            &target,
            Message::motion([3, 4], 0).unwrap(),
            nonce,
            Default::default(),
        )
        .unwrap();
        let wire = publication.message().unwrap();
        let mut modal = Modal::default();
        modal.token = nonce;
        modal.destination = Some(target);
        modal.publication = Some(publication);
        CALLBACK.with(|slot| slot.set((nonce, false)));
        EFFECT.with(|slot| slot.set(Some(effect)));
        unsafe {
            SendMessageW(child.0, wire.id, Some(wire.wp), Some(wire.lp));
        }
        assert!(
            modal.publication.as_ref().unwrap().finished(),
            "source must confirm the returned operation"
        );
        // 这里只模拟 SendMessageCallback 的到达；FINISHED 来自真实源 hook，不伪造成功属性。
        CALLBACK.with(|slot| slot.set((nonce, true)));
        modal.check_result().unwrap();
        assert!(!modal.pending());
        assert!(!modal.rejected);
        assert!(modal.publication.is_none());
        assert_eq!(protocol::value(fixture.b.0, protocol::COMMIT), 0);
    }
}
