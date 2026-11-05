//! 真 child 的签名准入、本地交互与终止；不把未接线的路由/正式结果称为端到端产品。
use super::*;
use crate::{
    device_authorization_lease::Grant as DeviceGrant, device_identity::Identity,
    device_session_attempt::Attempt,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::json;
use sha2::{Digest, Sha256};
pub(super) const SOURCE: &str = r#"
let label, button, count = 0;
NeuroSurface.define({mount({root}) {
  label=document.createElement('span'); label.textContent='0';
  button=document.createElement('button'); button.id='refresh'; button.textContent='increment';
  button.addEventListener('click',()=>{label.textContent=String(++count)});
  root.replaceChildren(label,button);
}, suspend(){label.textContent='paused'}, resume(){label.textContent=String(count)}});
"#;
fn key() -> Ed25519KeyPair {
    Ed25519KeyPair::from_seed_unchecked(&[11; 32]).unwrap()
}
pub(super) fn grant(source: &str) -> wire::Grant {
    let time = now().unwrap();
    let hash = format!("{:x}", Sha256::digest(source.as_bytes()));
    wire::Grant {
        protocol_version: wire::PROTOCOL.into(),
        grant_id: "grant:signed-child".into(),
        binding: wire::Binding {
            origin: "https://localhost:1234".into(),
            request_id: "request:1".into(),
            device_id: "viewer:javascript".into(),
            workspace_id: "workspace:1".into(),
            instance_id: "instance:javascript".into(),
            generation: 7,
            art_id: "vendor/counter".into(),
            art_version: "1.0.0".into(),
            package_digest: "a".repeat(64),
            sdk_version: wire::SDK.into(),
        },
        execution_owner: "otter".into(),
        result_owner: "otter_workspace".into(),
        permissions: vec![wire::Permission::LocalState, wire::Permission::Preview],
        services: vec![],
        issued_at_ms: time - 1,
        expires_at_ms: time + 60_000,
        offline_until_ms: time + 30_000,
        source: wire::Source {
            resource_id: format!("sha256:{hash}"),
            digest: hash,
            size: source.len() as u64,
        },
    }
}
fn signed(g: &wire::Grant) -> Vec<u8> {
    let payload = serde_json::to_vec(g).unwrap();
    let signature = key().sign(&[wire::DOMAIN, payload.as_slice()].concat());
    serde_json::to_vec(&wire::Envelope {
        algorithm: "ed25519".into(),
        key_id: "fixture-issuer".into(),
        payload: BASE64.encode(payload),
        signature: BASE64.encode(signature.as_ref()),
    })
    .unwrap()
}
pub(super) fn lease() -> (DeviceGrant, Arc<Lease>) {
    let mut identity = Identity::generate().unwrap();
    identity.bind("viewer:javascript").unwrap();
    let session = Attempt::new(&identity, "https://localhost:1234")
        .unwrap()
        .challenge(
            200,
            br#"{"deviceId":"viewer:javascript","challengeId":"id","challenge":"value"}"#,
        )
        .unwrap()
        .finish(
            200,
            &serde_json::to_vec(&json!({"deviceId":"viewer:javascript","token":"test-only",
            "expiresAtMs":now().unwrap()+60_000}))
            .unwrap(),
            now().unwrap(),
        )
        .unwrap();
    let mut owner = DeviceGrant::new(session);
    let lease = Arc::new(
        owner
            .lease("https://localhost:1234", now().unwrap())
            .unwrap(),
    );
    (owner, lease)
}
pub(super) fn mount(
    g: &wire::Grant,
    source: &str,
    lease: Arc<Lease>,
    cancelled: &impl Fn() -> bool,
) -> Result<(Local, Frame), Error> {
    mount_in(g, source, lease, None, cancelled)
}
fn mount_in(
    g: &wire::Grant,
    source: &str,
    lease: Arc<Lease>,
    viewport: Option<crate::surface_viewport::Viewport>,
    cancelled: &impl Fn() -> bool,
) -> Result<(Local, Frame), Error> {
    let key = key();
    Local::mount(
        Input {
            images: Vec::new(),
            envelope: &signed(g),
            issuer: Issuer {
                key_id: "fixture-issuer",
                public_key: key.public_key().as_ref().try_into().unwrap(),
            },
            expected: Expected {
                binding: &g.binding,
                permissions: &g.permissions,
                services: &g.services,
            },
            source: source.as_bytes(),
            view: super::super::fixture::binding(),
            snapshot: super::super::fixture::snapshot(1, 0),
            resources: serde_json::json!({}),
            viewport,
        },
        lease,
        true,
        cancelled,
    )
}
#[test]
fn signed_mount_reads_supplied_viewport_before_first_frame_and_reclaims_child() {
    let _serial = super::super::fixture::SERIAL.lock().unwrap();
    let source = "NeuroSurface.define({mount({root}){root.style.boxSizing='border-box';root.style.width='100%';const s=document.createElement('span');s.textContent=JSON.stringify([root.getBoundingClientRect().width,devicePixelRatio]);root.replaceChildren(s)}});";
    let g = grant(source);
    let (_owner, lease) = lease();
    let viewport = crate::surface_viewport::Viewport::new([600, 400], 192).unwrap();
    let (mut local, frame) = mount_in(&g, source, lease, Some(viewport), &|| false).unwrap();
    assert_eq!(frame.viewport, Some(viewport));
    assert_eq!(super::super::fixture::text(&frame.scene), "[300,2]");
    let process = probe(&local);
    local.stop();
    assert!(process.exited());
}
fn probe(local: &Local) -> crate::bounded_worker_process::Probe {
    local
        .session
        .as_ref()
        .unwrap()
        .child
        .as_ref()
        .unwrap()
        .probe()
        .unwrap()
}
#[test]
fn signed_child_local_events_preserve_state_and_stop_reclaims_only_owned_process() {
    let _serial = super::super::fixture::SERIAL.lock().unwrap();
    let (_owner, lease) = lease();
    let g = grant(SOURCE);
    let (mut local, initial) = mount(&g, SOURCE, lease.clone(), &|| false).unwrap();
    let probe = probe(&local);
    assert_eq!(super::super::fixture::text(&initial.scene), "0");
    let button = initial.scene.root().children()[1].id().to_owned();
    for n in 1..=3 {
        let frame = local.click(&g.binding, true, &button, &|| false).unwrap();
        assert_eq!(super::super::fixture::text(&frame.scene), n.to_string());
        assert!(frame.events.is_empty());
    }
    assert_eq!(
        super::super::fixture::text(&local.suspend(&g.binding, false, &|| false).unwrap().scene),
        "paused"
    );
    assert_eq!(
        super::super::fixture::text(&local.resume(&g.binding, false, &|| false).unwrap().scene),
        "3"
    );
    local.stop();
    assert!(probe.exited());
    assert!(!lease.is_revoked());
    assert_eq!(
        local.refresh(&g.binding, true, &|| false),
        Err(Error::Stopped)
    );
    assert_eq!(super::super::CAPACITY.used(), 0);
}
#[test]
fn signed_child_revocation_rebinding_and_cancellation_never_deliver_another_frame() {
    let _serial = super::super::fixture::SERIAL.lock().unwrap();
    for scenario in 0..3 {
        let (_owner, lease) = lease();
        let g = grant(SOURCE);
        let (mut local, initial) = mount(&g, SOURCE, lease.clone(), &|| false).unwrap();
        let button = initial.scene.root().children()[1].id().to_owned();
        let probe = probe(&local);
        let mut current = g.binding.clone();
        if scenario == 0 {
            lease.revoke();
        }
        if scenario == 1 {
            current.request_id = "request:next".into();
        }
        assert!(local
            .click(&current, true, &button, &|| scenario == 2)
            .is_err());
        assert!(probe.exited());
        assert_eq!(
            local.refresh(&g.binding, true, &|| false),
            Err(Error::Stopped)
        );
    }
    assert_eq!(super::super::CAPACITY.used(), 0);
}
#[test]
fn signed_child_cancellation_during_exchange_is_terminal_and_discards_reply() {
    let _serial = super::super::fixture::SERIAL.lock().unwrap();
    let (_owner, lease) = lease();
    let g = grant(SOURCE);
    let (mut local, initial) = mount(&g, SOURCE, lease, &|| false).unwrap();
    let button = initial.scene.root().children()[1].id().to_owned();
    let probe = probe(&local);
    let polls = std::cell::Cell::new(0);
    assert!(local
        .click(&g.binding, true, &button, &|| {
            polls.set(polls.get() + 1);
            polls.get() >= 3
        })
        .is_err());
    assert!(polls.get() >= 3);
    assert!(probe.exited());
    assert_eq!(
        local.refresh(&g.binding, true, &|| false),
        Err(Error::Stopped)
    );
    assert_eq!(super::super::CAPACITY.used(), 0);
}
#[test]
fn signed_child_refuses_tampering_and_unimplemented_service_scope_before_spawn() {
    let _serial = super::super::fixture::SERIAL.lock().unwrap();
    let (_owner, lease) = lease();
    let mut g = grant(SOURCE);
    assert!(matches!(
        mount(&g, "while(true){}", lease.clone(), &|| false),
        Err(Error::Grant(grant::Error::Source))
    ));
    g.permissions.push(wire::Permission::LoomService);
    g.services.push("service:echo".into());
    assert!(matches!(
        mount(&g, SOURCE, lease, &|| false),
        Err(Error::Unsupported)
    ));
    assert_eq!(super::super::CAPACITY.used(), 0);
}
#[test]
fn signed_child_rejects_legacy_emits_and_drop_reaps_process() {
    let _serial = super::super::fixture::SERIAL.lock().unwrap();
    let (_owner, lease) = lease();
    let source = r#"NeuroSurface.define({mount({root,emit}){root.textContent='not a formal result';emit({nodeId:'refresh',event:'click',action:'refresh',class:'discrete',payload:{}});}});"#;
    assert!(matches!(
        mount(&grant(source), source, lease.clone(), &|| false),
        Err(Error::Unsupported)
    ));
    let (local, _) = mount(&grant(SOURCE), SOURCE, lease, &|| false).unwrap();
    let probe = probe(&local);
    drop(local);
    assert!(probe.exited());
    assert_eq!(super::super::CAPACITY.used(), 0);
}

#[test]
fn signed_child_mount_cancellation_and_offline_denial_release_capacity() {
    let _serial = super::super::fixture::SERIAL.lock().unwrap();
    let (_owner, lease) = lease();
    let mut g = grant(SOURCE);
    let polls = std::cell::Cell::new(0);
    assert!(mount(&g, SOURCE, lease.clone(), &|| {
        polls.set(polls.get() + 1);
        polls.get() >= 3
    })
    .is_err());
    assert!(polls.get() >= 3);
    assert_eq!(super::super::CAPACITY.used(), 0);
    g.offline_until_ms = 0;
    let (mut local, _) = mount(&g, SOURCE, lease, &|| false).unwrap();
    let probe = probe(&local);
    assert!(matches!(
        local.refresh(&g.binding, false, &|| false),
        Err(Error::Grant(grant::Error::Time))
    ));
    assert!(probe.exited());
    assert!(matches!(
        local.resume(&g.binding, true, &|| false),
        Err(Error::Stopped)
    ));
}

#[test]
fn signed_child_revocation_after_execution_discards_successful_frame() {
    let _serial = super::super::fixture::SERIAL.lock().unwrap();
    let (_owner, lease) = lease();
    let g = grant(SOURCE);
    let (mut local, initial) = mount(&g, SOURCE, lease.clone(), &|| false).unwrap();
    let button = initial.scene.root().children()[1].id().to_owned();
    let probe = probe(&local);
    let result = local.operate(&g.binding, true, &|| false, |session, stop| {
        let frame = session.click(&button, &stop)?;
        assert_eq!(super::super::fixture::text(&frame.scene), "1");
        lease.revoke();
        Ok(frame)
    });
    assert!(matches!(result, Err(Error::Lease)));
    assert!(probe.exited());
    assert_eq!(super::super::CAPACITY.used(), 0);
}

#[test]
fn signed_child_rejects_emit_but_returns_focus_only_as_native_host_intent() {
    let _serial = super::super::fixture::SERIAL.lock().unwrap();
    let (_owner, lease) = lease();
    for action in [
        "emit({nodeId:'refresh',event:'click',action:'refresh',class:'discrete',payload:{}})",
        "const field=document.createElement('input');root.appendChild(field);field.focus()",
    ] {
        let source = format!("NeuroSurface.define({{mount({{root,emit}}){{const button=document.createElement('button');button.id='refresh';button.textContent='run';button.addEventListener('click',()=>{{{action}}});root.appendChild(button)}}}});");
        let g = grant(&source);
        let (mut local, initial) = mount(&g, &source, lease.clone(), &|| false).unwrap();
        let button = initial.scene.root().children()[0].id().to_owned();
        let probe = probe(&local);
        let result = local.click(&g.binding, true, &button, &|| false);
        if action.starts_with("emit") {
            assert_eq!(result.err(), Some(Error::Unsupported));
        } else {
            let frame = result.unwrap();
            assert_eq!(frame.focus.len(), 1);
            assert!(frame.events.is_empty());
            assert!(!probe.exited());
            local.stop();
        }
        assert!(probe.exited());
    }
    assert_eq!(super::super::CAPACITY.used(), 0);
}
