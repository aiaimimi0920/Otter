//! 内联请求的完整场景身份、Grant、取消与共享容量门。
use super::*;
use crate::surface_resource_runtime::request::{inline::Inline, Key, Request};
fn mount() -> MountedSurface {
    use base64::Engine;
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[1, 2, 3, 255])
            .unwrap();
    }
    let (mut current, _) = image("https://localhost:1234", b"data");
    mutate(
        &mut current,
        |s| s["scene"] = serde_json::json!({"id":"image","type":"image","props":{"src":format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))}}),
    );
    current
}
fn inline_ready(current: &MountedSurface, grant: &mut Grant) -> Ready {
    let request = Inline::from_mount(current, "image").unwrap();
    let mut job = Job::start_inline(
        grant.lease(current.binding.origin(), clock()).unwrap(),
        request,
    )
    .unwrap();
    wait(&job);
    job.try_join().unwrap().unwrap()
}
#[test]
fn inline_image_source_identity_and_priority_are_exact() {
    let mut current = mount();
    let request = Inline::from_mount(&current, "image").unwrap();
    assert_ne!(Key::Node("image".into()), Key::Resource("image".into()));
    request.matches(&current).unwrap();
    mutate(&mut current, |s| {
        s["scene"]["props"]["src"] = "data:image/png,YQ==".into()
    });
    assert_eq!(request.matches(&current), Err(ResourceError::Identity));
    let mut current = mount();
    mutate(&mut current, |s| s["revision"] = 4.into());
    assert_eq!(request.matches(&current), Err(ResourceError::Identity));
    let mut current = mount();
    mutate(&mut current, |s| {
        s["scene"]["props"]["resourceId"] = "missing".into()
    });
    assert!(Inline::from_mount(&current, "image").is_err());
    for value in [serde_json::json!(""), serde_json::json!(false)] {
        mutate(&mut current, |s| s["scene"]["props"]["resourceId"] = value);
        assert!(Inline::from_mount(&current, "image").is_ok());
    }
    let mut current = mount();
    mutate(&mut current, |s| {
        let child = s["scene"].clone();
        s["scene"] = serde_json::json!({"id":"outer","type":"image","children":[child]});
    });
    assert!(Inline::from_mount(&current, "image").is_err());
}
#[test]
fn inline_image_ready_transfer_requires_same_grant_and_releases_capacity() {
    let current = mount();
    let mut grant = Grant::new(session(current.binding.origin(), u64::MAX));
    let ready = inline_ready(&current, &mut grant);
    let held: Vec<_> = (0..3)
        .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
        .collect();
    assert!(crate::live_relay_control_runtime::Slot::acquire().is_err());
    let destination = grant.lease(current.binding.origin(), clock()).unwrap();
    let (request, image, logical_size) = ready
        .transfer_image(&destination, &current, clock())
        .unwrap();
    assert!(matches!(request, Request::Inline(_)));
    assert_eq!(logical_size, [1., 1.]);
    assert_eq!(&*image.pixels, &[3, 2, 1, 255]);
    let freed = crate::live_relay_control_runtime::Slot::acquire().unwrap();
    request.validate(&destination, &current, clock()).unwrap();
    drop(freed);
    drop(held);
    let ready = inline_ready(&current, &mut grant);
    let mut other = Grant::new(session(current.binding.origin(), u64::MAX));
    assert!(matches!(
        ready.transfer_image(
            &other.lease(current.binding.origin(), clock()).unwrap(),
            &current,
            clock()
        ),
        Err(JobError::Resource(ResourceError::Identity))
    ));
    drop(grant);
    assert!(request.validate(&destination, &current, clock()).is_err());
}
#[test]
fn inline_image_ready_cancel_and_changed_source_reject_delivery() {
    let mut current = mount();
    let mut grant = Grant::new(session(current.binding.origin(), u64::MAX));
    let ready = inline_ready(&current, &mut grant);
    mutate(&mut current, |s| {
        s["scene"]["props"]["src"] = "data:image/png,YQ==".into()
    });
    assert!(ready.image(&current, clock()).is_err());
    ready.request_stop();
    assert!(matches!(
        ready.image(&mount(), clock()),
        Err(JobError::Stopped)
    ));
    drop(ready);
    let slots: Vec<_> = (0..4)
        .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
        .collect();
    assert_eq!(slots.len(), 4);
}
