//! 稳定输出身份、有限几何与终态意图；真实换代/线程排空由LAN联合保护。
use super::*;

#[test]
fn inventory_requires_exact_unique_identity_and_never_follows_another_monitor() {
    let original = crate::tile_tests::output();
    let mut owner = Output::new(&original.output_id);
    owner.observe(Ok(vec![original.clone()]));
    assert_eq!(owner.observed, Some(original.clone()));
    let mut other = original.clone();
    other.output_id = format!("monitor-{}", "b".repeat(64));
    for inventory in [
        Err("enumeration unavailable".into()),
        Ok(vec![]),
        Ok(vec![other.clone()]),
        Ok(vec![original.clone(), original.clone()]),
    ] {
        owner.observe(inventory);
        assert!(owner.observed.is_none());
    }
    owner.observe(Ok(vec![other, original.clone()]));
    assert_eq!(owner.observed, Some(original));
}

#[test]
fn inventory_rejects_zero_axes_pixel_excess_and_coordinate_overflow() {
    let original = crate::tile_tests::output();
    let mut owner = Output::new(&original.output_id);
    for (width, height, x, y) in [
        (0, 2, 0, 0),
        (2, 0, 0, 0),
        (16385, 1, 0, 0),
        (1, 16385, 0, 0),
        (4097, 4096, 0, 0),
        (2, 2, i32::MAX, 0),
        (2, 2, 0, i32::MAX),
    ] {
        let mut output = original.clone();
        (output.width, output.height, output.x, output.y) = (width, height, x, y);
        owner.observe(Ok(vec![output]));
        assert!(owner.observed.is_none(), "{width}x{height}@{x},{y}");
    }
    let mut edge = original;
    (edge.width, edge.height, edge.x, edge.y) = (4096, 4096, i32::MIN, i32::MIN);
    owner.observe(Ok(vec![edge.clone()]));
    assert_eq!(owner.observed, Some(edge));
}

#[test]
fn stop_is_sticky_even_if_original_monitor_returns_without_authorization() {
    let original = crate::tile_tests::output();
    let mut output = Output::new(&original.output_id);
    output.observe(Ok(vec![original.clone()]));
    output.stop();
    output.observe(Ok(vec![original]));
    assert!(output.observed.is_none() && output.drain());
    let mut owner = Owner::new();
    assert_eq!(
        output
            .tick(&mut owner, "https://127.0.0.1:1", None)
            .unwrap(),
        Status::Stopped
    );
    assert!(output.active.is_none() && output.retiring.is_none());
}

#[test]
fn missing_authorization_is_terminal_not_an_implicit_pairing_or_recovery() {
    let original = crate::tile_tests::output();
    let mut output = Output::new(&original.output_id);
    output.observe(Ok(vec![original.clone()]));
    let mut owner = Owner::new();
    assert!(output
        .tick(&mut owner, "https://127.0.0.1:1", None)
        .is_err());
    output.observe(Ok(vec![original]));
    assert_eq!(
        output
            .tick(&mut owner, "https://127.0.0.1:1", None)
            .unwrap(),
        Status::Stopped
    );
    assert!(owner.finished());
}
