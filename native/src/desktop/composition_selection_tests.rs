//! 故障注入验证完整候选选择、有限次数和清理顺序；真实 WARP 像素在 composition_tests 中验证。
use super::*;
use std::{cell::RefCell, rc::Rc};
use windows::Win32::Foundation::{E_ABORT, E_FAIL};

#[test]
fn unconfirmed_association_denies_warp_and_preserves_original_error() {
    let mut calls = Vec::new();
    let result = select::<()>(|backend| {
        calls.push(backend);
        Err((
            Stage::Presentation,
            composition_creation::Failure::attached(Error::new(
                E_ABORT,
                "partial hardware association",
            )),
        ))
    });
    assert_eq!(calls, [Backend::D3d11Hardware]);
    let error = result.err().unwrap().into_error();
    assert_eq!(error.code(), E_ABORT);
    assert!(error.message().contains("partial hardware association"));
}

#[test]
fn successful_primary_never_creates_a_second_backend() {
    let mut calls = Vec::new();
    let selected = select(|backend| {
        calls.push(backend);
        Ok(7)
    })
    .unwrap();
    assert_eq!(calls, [Backend::D3d11Hardware]);
    assert_eq!(selected.backend, Backend::D3d11Hardware);
    assert!(selected.fallback.is_none());
    assert_eq!(selected.value, 7);
}

#[test]
fn device_and_presentation_failures_both_allow_one_software_candidate() {
    for stage in [Stage::Device, Stage::Presentation] {
        let mut calls = Vec::new();
        let selected = select(|backend| {
            calls.push(backend);
            if backend == Backend::D3d11Hardware {
                Err((stage, Error::from(E_ABORT).into()))
            } else {
                Ok(9)
            }
        })
        .unwrap();
        assert_eq!(calls, [Backend::D3d11Hardware, Backend::D3d11Warp]);
        assert_eq!(selected.backend, Backend::D3d11Warp);
        assert_eq!(selected.value, 9);
        assert_eq!(
            selected.fallback,
            Some(Failure {
                backend: Backend::D3d11Hardware,
                stage,
                code: E_ABORT
            })
        );
    }
}

#[test]
fn final_failure_keeps_both_stage_codes_and_does_not_retry() {
    let mut calls = Vec::new();
    let result = select::<()>(|backend| {
        calls.push(backend);
        Err(if backend == Backend::D3d11Hardware {
            (Stage::Presentation, Error::from(E_ABORT).into())
        } else {
            (Stage::Device, Error::from(E_FAIL).into())
        })
    });
    let error = result.err().unwrap().into_error();
    assert_eq!(error.code(), E_FAIL);
    let message = error.message();
    for fragment in [
        "D3d11Hardware",
        "Presentation",
        "80004004",
        "D3d11Warp",
        "Device",
        "80004005",
    ] {
        assert!(message.contains(fragment), "missing {fragment}: {message}");
    }
    assert_eq!(calls.len(), 2);
}

struct Candidate(Rc<RefCell<Vec<&'static str>>>);
impl Drop for Candidate {
    fn drop(&mut self) {
        self.0.borrow_mut().push("released");
    }
}
#[test]
fn failed_candidate_resources_are_released_before_fallback_and_success_is_owned() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let selected = select(|backend| {
        if backend == Backend::D3d11Hardware {
            let _partial = Candidate(events.clone());
            events.borrow_mut().push("hardware");
            return Err((Stage::Presentation, Error::from(E_FAIL).into()));
        }
        assert_eq!(*events.borrow(), ["hardware", "released"]);
        events.borrow_mut().push("software");
        Ok(Candidate(events.clone()))
    })
    .unwrap();
    assert_eq!(*events.borrow(), ["hardware", "released", "software"]);
    drop(selected);
    assert_eq!(
        *events.borrow(),
        ["hardware", "released", "software", "released"]
    );
}
