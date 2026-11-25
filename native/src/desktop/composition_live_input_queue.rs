//! HWND 待发送队列：只合并尚未投递的等价连续样本，保留边沿顺序和硬容量。
use crate::live_input::Event;
use std::collections::VecDeque;

// 一个 Win32 signed WORD 滚轮包最多拆成 28 个合法内部包；容量仍是固定上限。
pub(super) const CAPACITY: usize = 32;
const PACKET_LIMIT: i32 = 1200;

/// 本地 HWND 入口接受完整 signed WORD；内部/远程事件的包上限保持不变。
pub(super) fn push_native(events: &mut VecDeque<Event>, event: Event, buttons: u8) -> bool {
    let Event::Wheel {
        point,
        delta,
        horizontal,
    } = event
    else {
        return push_nonwheel(events, event);
    };
    if i16::try_from(delta).is_err()
        || !(Event::Wheel {
            point,
            delta: delta.signum(),
            horizontal,
        })
        .valid()
    {
        return false;
    }

    // 同一点的静止 move 可让位给同向同轴的滚轮，但最后仍留在队尾；拖动不重排。
    let has_motion = buttons == 0 && events.back() == Some(&Event::Move { point });
    let compatible = match events.iter_mut().rev().nth(usize::from(has_motion)) {
        Some(Event::Wheel {
            point: previous,
            delta: tail,
            horizontal: axis,
        }) if *previous == point && *axis == horizontal && tail.signum() == delta.signum() => {
            Some(tail)
        }
        _ => None,
    };
    let move_to_tail = has_motion && compatible.is_some();
    if let Some(tail) = compatible {
        if let Some(total) = tail
            .checked_add(delta)
            .filter(|total| total.unsigned_abs() <= PACKET_LIMIT as u32)
        {
            *tail = total;
            return true;
        }
    }

    // 首个满包不能与同向非零尾包合并，后续余量也不会与满包合并。
    // 静止 move 取下后还要放回，因此净增量恰好为新包数。容量不足时队列未动。
    let packets = delta.unsigned_abs().div_ceil(PACKET_LIMIT as u32) as usize;
    if packets > CAPACITY - events.len() {
        return false;
    }
    let motion = if move_to_tail {
        events.pop_back()
    } else {
        None
    };
    let mut remaining = delta;
    while remaining != 0 {
        let part = remaining.clamp(-PACKET_LIMIT, PACKET_LIMIT);
        events.push_back(Event::Wheel {
            point,
            delta: part,
            horizontal,
        });
        remaining -= part;
    }
    events.extend(motion);
    true
}

/// 只有合法多格滚轮需要查询浏览器类别；非法包原样交给 Controller 拒绝。
pub(super) fn pop_for_dispatch(
    events: &mut VecDeque<Event>,
    is_browser: impl FnOnce() -> bool,
) -> Option<Event> {
    let front = events.front_mut()?;
    match *front {
        event @ Event::Wheel {
            point,
            delta,
            horizontal,
        } if delta.unsigned_abs() > 120 && event.valid() && is_browser() => {
            // 未发送余量留在原队首，不扩容、不越过后续边沿。
            let part = delta.signum() * 120;
            *front = Event::Wheel {
                point,
                delta: delta - part,
                horizontal,
            };
            Some(Event::Wheel {
                point,
                delta: part,
                horizontal,
            })
        }
        _ => events.pop_front(),
    }
}

fn push_nonwheel(events: &mut VecDeque<Event>, event: Event) -> bool {
    if event == Event::Leave && events.back() == Some(&Event::Leave) {
        return true;
    }
    if matches!(event, Event::Move { .. }) && matches!(events.back(), Some(Event::Move { .. })) {
        *events.back_mut().unwrap() = event;
        return true;
    }
    if events.len() == CAPACITY {
        return false;
    }
    events.push_back(event);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn push(events: &mut VecDeque<Event>, event: Event) -> bool {
        push_native(events, event, 0)
    }

    fn wheel(delta: i32) -> Event {
        Event::Wheel {
            point: [0.5, 0.5],
            delta,
            horizontal: false,
        }
    }
    #[test]
    fn native_large_packets_preserve_full_signed_word_without_relaxing_event_contract() {
        for delta in [-32768, -2400, 2400, 32767] {
            let mut events = VecDeque::new();
            assert!(!wheel(delta).valid());
            assert!(push_native(&mut events, wheel(delta), 0));
            assert!(events.iter().all(|event| event.valid()));
            assert!(events.len() <= CAPACITY);
            let mut total = 0;
            while let Some(Event::Wheel { delta, .. }) = pop_for_dispatch(&mut events, || true) {
                total += delta;
            }
            assert_eq!(total, delta);
        }
    }
    #[test]
    fn native_stationary_motion_does_not_discard_fast_wheel_distance() {
        let mut events = VecDeque::new();
        for _ in 0..100 {
            assert!(push_native(
                &mut events,
                Event::Move { point: [0.5, 0.5] },
                0
            ));
            assert!(push_native(&mut events, wheel(-120), 0));
        }
        assert_eq!(events.len(), 12); // initial move, ten wheel packets, final move
        assert_eq!(events.front(), Some(&Event::Move { point: [0.5, 0.5] }));
        assert_eq!(events.back(), Some(&Event::Move { point: [0.5, 0.5] }));
        assert_eq!(
            events
                .iter()
                .filter_map(|event| match event {
                    Event::Wheel { delta, .. } => Some(*delta),
                    _ => None,
                })
                .sum::<i32>(),
            -12000
        );
    }
    #[test]
    fn native_motion_coalescing_preserves_target_direction_axis_and_drag_boundaries() {
        for (motion, next, buttons) in [
            (Event::Move { point: [0.6, 0.5] }, wheel(-120), 0),
            (Event::Move { point: [0.5, 0.5] }, wheel(120), 0),
            (
                Event::Move { point: [0.5, 0.5] },
                Event::Wheel {
                    point: [0.5, 0.5],
                    delta: -120,
                    horizontal: true,
                },
                0,
            ),
            (Event::Move { point: [0.5, 0.5] }, wheel(-120), 1),
            (Event::Leave, wheel(-120), 0),
        ] {
            let mut events = VecDeque::from([wheel(-120), motion]);
            assert!(push_native(&mut events, next, buttons));
            assert_eq!(events, VecDeque::from([wheel(-120), motion, next]));
        }
    }
    #[test]
    fn native_large_packet_admission_is_atomic_and_rejects_non_native_values() {
        let mut events = VecDeque::from(vec![Event::Leave; CAPACITY - 1]);
        let before = events.clone();
        assert!(!push_native(&mut events, wheel(-2400), 0));
        assert_eq!(events, before);
        for delta in [0, -32769, 32768, i32::MAX, i32::MIN] {
            assert!(!push_native(&mut events, wheel(delta), 0));
            assert_eq!(events, before);
        }
    }
    #[test]
    fn full_queue_merges_before_stationary_move_or_rejects_without_touching_it() {
        let motion = Event::Move { point: [0.5, 0.5] };
        let mut events = VecDeque::from(vec![Event::Character { unit: 65 }; CAPACITY - 2]);
        events.extend([wheel(-1100), motion]);
        assert!(push_native(&mut events, wheel(-100), 0));
        assert_eq!(events[CAPACITY - 2], wheel(-1200));
        assert_eq!(events.back(), Some(&motion));
        let snapshot = events.clone();
        for delta in [-1, -1201, -32768] {
            assert!(!push_native(&mut events, wheel(delta), 0));
            assert_eq!(events, snapshot);
        }
    }

    #[test]
    fn large_packet_preserves_full_chunks_and_stationary_motion_at_exact_capacity() {
        let motion = Event::Move { point: [0.5, 0.5] };
        let mut events = VecDeque::with_capacity(CAPACITY);
        events.extend(std::iter::repeat_n(
            Event::Character { unit: 65 },
            CAPACITY - 4,
        ));
        events.extend([wheel(30), motion]);
        assert!(push_native(&mut events, wheel(1201), 0));
        assert_eq!(events.len(), CAPACITY);
        assert_eq!(
            events.range(CAPACITY - 4..).copied().collect::<Vec<_>>(),
            [wheel(30), wheel(1200), wheel(1), motion]
        );
    }

    #[test]
    fn ordinary_wheel_reuses_reserved_queue_storage() {
        let mut events = VecDeque::with_capacity(CAPACITY);
        events.push_back(Event::Character { unit: 65 });
        let allocation = events.as_slices().0.as_ptr();
        for delta in [120, 30, -120] {
            assert!(push_native(&mut events, wheel(delta), 0));
            assert_eq!(events.as_slices().0.as_ptr(), allocation);
        }
        assert_eq!(
            events,
            [Event::Character { unit: 65 }, wheel(150), wheel(-120)]
        );
    }

    #[test]
    fn wheel_merging_preserves_sum_and_packet_limit() {
        let mut events = VecDeque::new();
        for _ in 0..100 {
            assert!(push(&mut events, wheel(-30)));
        }
        assert_eq!(
            events,
            VecDeque::from([wheel(-1200), wheel(-1200), wheel(-600)])
        );
    }
    #[test]
    fn browser_dispatch_splits_ticks_without_expanding_queue_or_crossing_edges() {
        for delta in [-1200, -301, -30, 30, 301, 1200] {
            for horizontal in [false, true] {
                let event = Event::Wheel {
                    point: [0.3, 0.7],
                    delta,
                    horizontal,
                };
                let key = Event::Key {
                    code: 65,
                    down: true,
                };
                let mut events = VecDeque::from([event, key]);
                let mut total = 0;
                while let Some(Event::Wheel { .. }) = events.front() {
                    let Some(Event::Wheel {
                        point,
                        delta: part,
                        horizontal: axis,
                    }) = pop_for_dispatch(&mut events, || true)
                    else {
                        panic!("wheel must precede key")
                    };
                    assert_eq!(point, [0.3, 0.7]);
                    assert_eq!(axis, horizontal);
                    assert_eq!(part.signum(), delta.signum());
                    assert!(part.unsigned_abs() <= 120);
                    assert!(events.len() <= 2);
                    total += part;
                }
                assert_eq!(total, delta);
                assert_eq!(pop_for_dispatch(&mut events, || true), Some(key));
                assert_eq!(pop_for_dispatch(&mut events, || true), None);
            }
        }
    }
    #[test]
    fn browser_cancel_drops_only_unsent_remainder_and_merge_never_replays_sent_tick() {
        let mut events = VecDeque::from([wheel(-300)]);
        assert_eq!(pop_for_dispatch(&mut events, || true), Some(wheel(-120)));
        assert_eq!(events.front(), Some(&wheel(-180)));
        assert!(push(&mut events, wheel(-60)));
        assert_eq!(pop_for_dispatch(&mut events, || true), Some(wheel(-120)));
        assert_eq!(pop_for_dispatch(&mut events, || true), Some(wheel(-120)));
        assert!(events.is_empty());
        assert!(push(&mut events, wheel(1200)));
        assert_eq!(pop_for_dispatch(&mut events, || true), Some(wheel(120)));
        events.clear();
        assert_eq!(pop_for_dispatch(&mut events, || true), None);
    }
    #[test]
    fn dispatch_skips_browser_query_when_splitting_cannot_change_the_result() {
        let invalid_point = Event::Wheel {
            point: [2.0, 0.5],
            delta: 1200,
            horizontal: false,
        };
        let inputs = [
            Event::Move { point: [0.5, 0.5] },
            Event::Leave,
            wheel(120),
            wheel(-120),
            wheel(30),
            wheel(0),
            wheel(1201),
            wheel(-1201),
            wheel(i32::MIN),
            wheel(i32::MAX),
            invalid_point,
        ];
        let mut events = VecDeque::from(inputs);
        let query = || panic!("this event cannot be split");
        for event in inputs {
            assert_eq!(pop_for_dispatch(&mut events, query), Some(event));
        }
        assert_eq!(pop_for_dispatch(&mut events, query), None);
    }

    #[test]
    fn dispatch_queries_only_multitick_front_and_preserves_nonbrowser_packet() {
        let key = Event::Key {
            code: 65,
            down: true,
        };
        for (browser, first) in [(true, wheel(-120)), (false, wheel(-240))] {
            let mut events = VecDeque::from([wheel(-240), key]);
            let mut calls = 0;
            assert_eq!(
                pop_for_dispatch(&mut events, || {
                    calls += 1;
                    browser
                }),
                Some(first)
            );
            assert_eq!(calls, 1);
            let query = || panic!("remainder and following key do not require a query");
            if browser {
                assert_eq!(pop_for_dispatch(&mut events, query), Some(wheel(-120)));
            }
            assert_eq!(pop_for_dispatch(&mut events, query), Some(key));
            assert!(events.is_empty());
        }
    }
    #[test]
    fn wheel_never_merges_across_semantic_boundaries() {
        for boundary in [
            wheel(-30),
            Event::Wheel {
                point: [0.6, 0.5],
                delta: 30,
                horizontal: false,
            },
            Event::Wheel {
                point: [0.5, 0.5],
                delta: 30,
                horizontal: true,
            },
            Event::Key {
                code: 65,
                down: true,
            },
            Event::Move { point: [0.6, 0.5] },
            Event::Leave,
        ] {
            let mut events = VecDeque::new();
            for event in [wheel(30), boundary, wheel(30)] {
                assert!(push(&mut events, event));
            }
            assert_eq!(events, VecDeque::from([wheel(30), boundary, wheel(30)]));
        }
    }
    #[test]
    fn full_queue_only_accepts_equivalent_tail_and_never_drops_edges() {
        let mut events = VecDeque::from(vec![
            Event::Key {
                code: 65,
                down: true
            };
            CAPACITY - 1
        ]);
        assert!(push(&mut events, wheel(30)));
        assert!(push(&mut events, wheel(30)));
        let before = events.clone();
        assert!(!push(
            &mut events,
            Event::Key {
                code: 65,
                down: false
            }
        ));
        assert_eq!(events, before);
        assert!(!push(&mut events, wheel(1200)));
        assert_eq!(events, before);
    }
}
