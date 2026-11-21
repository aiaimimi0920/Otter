//! HWND 待发送队列：只合并尚未投递的等价连续样本，保留边沿顺序和硬容量。
use crate::live_input::Event;
use std::collections::VecDeque;

// 一个 Win32 signed WORD 滚轮包最多拆成 28 个合法内部包；容量仍是固定上限。
pub(super) const CAPACITY: usize = 32;

/// 本地 HWND 入口接受完整 signed WORD；内部/远程事件的 1200 契约保持不变。
pub(super) fn push_native(events: &mut VecDeque<Event>, event: Event, buttons: u8) -> bool {
    let Event::Wheel {
        point,
        delta,
        horizontal,
    } = event
    else {
        return push(events, event);
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
    // 原子准入，不能只留下大包的一部分。最多复制固定容量，不保留输入历史。
    let mut next = events.clone();
    let motion = if buttons == 0
        && next.back() == Some(&Event::Move { point })
        && next.iter().rev().nth(1).is_some_and(|event| {
            matches!(event,
            Event::Wheel { point: p, delta: d, horizontal: h }
            if *p == point && *h == horizontal && d.signum() == delta.signum())
        }) {
        // 同一点重复 move 不代表换目标；保留最后一次 move，避免阻断同向滚轮合并。
        next.pop_back()
    } else {
        None
    };
    let mut remaining = delta;
    while remaining != 0 {
        let part = remaining.clamp(-1200, 1200);
        if !push(
            &mut next,
            Event::Wheel {
                point,
                delta: part,
                horizontal,
            },
        ) {
            return false;
        }
        remaining -= part;
    }
    if let Some(motion) = motion {
        if !push(&mut next, motion) {
            return false;
        }
    }
    *events = next;
    true
}

/// 浏览器每次只授权一格；未发送余量留在原队首，不扩容、不越过后续边沿。
pub(super) fn pop_browser(events: &mut VecDeque<Event>) -> Option<Event> {
    // 非法原包仍交给原 Controller 拒绝，不能分片后绕过幅度或坐标契约。
    if events.front().is_some_and(|event| !event.valid()) {
        return events.pop_front();
    }
    if let Some(Event::Wheel {
        point,
        delta,
        horizontal,
    }) = events.front_mut()
    {
        if delta.unsigned_abs() > 120 {
            let part = delta.signum() * 120;
            *delta -= part;
            return Some(Event::Wheel {
                point: *point,
                delta: part,
                horizontal: *horizontal,
            });
        }
    }
    events.pop_front()
}

pub(super) fn push(events: &mut VecDeque<Event>, event: Event) -> bool {
    if event == Event::Leave && events.back() == Some(&Event::Leave) {
        return true;
    }
    if matches!(event, Event::Move { .. }) && matches!(events.back(), Some(Event::Move { .. })) {
        *events.back_mut().unwrap() = event;
        return true;
    }
    if let (
        Some(Event::Wheel {
            point,
            delta,
            horizontal,
        }),
        Event::Wheel {
            point: next_point,
            delta: next_delta,
            horizontal: next_horizontal,
        },
    ) = (events.back_mut(), event)
    {
        // 不跨方向、坐标、轴或其他输入边沿合并；每包沿用 Controller 的 1200 上限。
        // 已在途/已执行的消息不在这里，不能因合并重放未知结果。
        if *point == next_point
            && *horizontal == next_horizontal
            && delta.signum() == next_delta.signum()
        {
            if let Some(total) = delta
                .checked_add(next_delta)
                .filter(|v| v.unsigned_abs() <= 1200)
            {
                *delta = total;
                return true;
            }
        }
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
            while let Some(Event::Wheel { delta, .. }) = pop_browser(&mut events) {
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
                    }) = pop_browser(&mut events)
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
                assert_eq!(pop_browser(&mut events), Some(key));
                assert_eq!(pop_browser(&mut events), None);
            }
        }
    }
    #[test]
    fn browser_cancel_drops_only_unsent_remainder_and_merge_never_replays_sent_tick() {
        let mut events = VecDeque::from([wheel(-300)]);
        assert_eq!(pop_browser(&mut events), Some(wheel(-120)));
        assert_eq!(events.front(), Some(&wheel(-180)));
        assert!(push(&mut events, wheel(-60)));
        assert_eq!(pop_browser(&mut events), Some(wheel(-120)));
        assert_eq!(pop_browser(&mut events), Some(wheel(-120)));
        assert!(events.is_empty());
        assert!(push(&mut events, wheel(1200)));
        assert_eq!(pop_browser(&mut events), Some(wheel(120)));
        events.clear();
        assert_eq!(pop_browser(&mut events), None);
    }
    #[test]
    fn browser_dispatch_preserves_rejection_of_invalid_original_packets() {
        for delta in [0, 1201, -1201, i32::MIN, i32::MAX] {
            let event = wheel(delta);
            let mut events = VecDeque::from([event]);
            assert_eq!(pop_browser(&mut events), Some(event));
            assert!(events.is_empty());
            assert!(!event.valid());
        }
        let event = Event::Wheel {
            point: [2.0, 0.5],
            delta: 1200,
            horizontal: false,
        };
        let mut events = VecDeque::from([event]);
        assert_eq!(pop_browser(&mut events), Some(event));
        assert!(events.is_empty());
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
            Event::Move { point: [0.5, 0.5] },
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
