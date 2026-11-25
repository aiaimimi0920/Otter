//! 模态准入只观察一次：注入回调到达的值状态，不模拟源 FINISHED 或调用产品 DLL。
use super::*;
use crate::live_window_target::recovery::popup_layer::fixture::Fixture;
use otter_live_hover_hook::native_protocol as protocol;
#[path = "live_input_modal_routing_tests.rs"]
pub mod routing_tests;

struct CallbackReset;
impl CallbackReset {
    fn new() -> Self {
        assert_eq!(CALLBACK.with(Cell::get), (0, false));
        Self
    }
}
impl Drop for CallbackReset {
    fn drop(&mut self) {
        CALLBACK.with(|slot| slot.set((0, false)));
    }
}
fn publication(fixture: &Fixture, nonce: usize) -> (Modal, Destination) {
    let root = Target::bind(fixture.root.0).unwrap();
    let target = Destination::bind(&root, root.hwnd()).unwrap();
    let control = fixture.owner.identity().capture_control().unwrap();
    let mut modal = Modal::default();
    modal.set_owner(control);
    modal.token = nonce;
    modal.destination = Some(target.clone());
    modal.publication = Some(
        native_request::Publication::publish(
            control,
            &target,
            Message::motion([2, 3], 0).unwrap(),
            nonce,
            Default::default(),
        )
        .unwrap(),
    );
    CALLBACK.with(|slot| slot.set((nonce, false)));
    (modal, target)
}

#[test]
fn callback_arriving_after_the_result_check_cannot_turn_a_busy_slot_into_admission() {
    let _reset = CallbackReset::new();
    let fixture = Fixture::new();
    let (mut modal, target) = publication(&fixture, 601);
    modal.check_result().unwrap();
    // 模拟上次观察之后回调才到达；不写 FINISHED，也不再次泵消息。
    unsafe { completed(target.hwnd(), 0, 601, LRESULT(0)) };
    assert_eq!(modal.require_idle(), Err(Error::ModalPending));
    assert_eq!(modal.token, 601);
    assert!(modal.publication.is_some());
    assert_eq!(CALLBACK.with(Cell::get), (601, true));
    assert_eq!(protocol::value(fixture.b.0, protocol::COMMIT), 601);
    assert_eq!(protocol::value(fixture.b.0, protocol::FINISHED), 0);
    assert_eq!(modal.check_result(), Err(Error::DeliveryFailed));
    assert_eq!(modal.token, 0);
    assert!(modal.publication.is_none());
    assert_eq!(protocol::value(fixture.b.0, protocol::COMMIT), 0);
    for packet in [
        Message::button([2, 3], 1, crate::live_input::Button::Left, true, false).unwrap(),
        Message::button([2, 3], 0, crate::live_input::Button::Right, false, false).unwrap(),
    ] {
        assert_eq!(
            button_modal(&target, packet, &mut modal),
            Err(Error::DeliveryFailed)
        );
    }
    assert_eq!(
        motion_native(
            &target,
            Message::motion([2, 3], 0).unwrap(),
            250,
            &mut modal
        ),
        Err(Error::DeliveryFailed)
    );
    assert_eq!(protocol::value(fixture.b.0, protocol::COMMIT), 0);
}

#[test]
fn pending_escape_and_foreign_callback_slots_remain_unavailable_without_observation() {
    let _reset = CallbackReset::new();
    let mut modal = Modal::default();
    assert_eq!(modal.require_idle(), Ok(()));
    modal.token = 602;
    CALLBACK.with(|slot| slot.set((602, false)));
    assert_eq!(modal.require_idle(), Err(Error::ModalPending));
    modal.clear();
    modal.escape_edges = 1;
    assert_eq!(modal.require_idle(), Err(Error::ModalPending));
    modal.clear();
    CALLBACK.with(|slot| slot.set((603, true)));
    assert_eq!(modal.require_idle(), Err(Error::ModalPending));
    assert_eq!(CALLBACK.with(Cell::get), (603, true));
    CALLBACK.with(|slot| slot.set((0, false)));
    assert_eq!(modal.require_idle(), Ok(()));
}

#[test]
fn confirmed_completion_reopens_the_slot_but_does_not_drop_a_pending_escape_release() {
    let _reset = CallbackReset::new();
    let fixture = Fixture::new();
    for (nonce, escape) in [(604, 0), (605, 1)] {
        let (mut modal, target) = publication(&fixture, nonce);
        modal.escape_edges = escape;
        let wire = modal.publication.as_ref().unwrap().message().unwrap();
        unsafe { SendMessageW(target.hwnd(), wire.id, Some(wire.wp), Some(wire.lp)) };
        assert!(modal.publication.as_ref().unwrap().finished());
        // FINISHED 由实际自有源 hook 写入，这里只模拟回调到达。
        unsafe { completed(target.hwnd(), wire.id, nonce, LRESULT(0)) };
        modal.check_result().unwrap();
        assert_eq!(modal.token, 0);
        assert_eq!(
            modal.require_idle(),
            if escape == 1 {
                Err(Error::ModalPending)
            } else {
                Ok(())
            }
        );
        assert_eq!(modal.destination.is_some(), escape == 1);
        assert_eq!(protocol::value(fixture.b.0, protocol::COMMIT), 0);
    }
}
