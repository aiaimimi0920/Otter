//! 显式隔离的真实 Loom 联合驱动；只通过生产 Lease 接口操作，不接受 token 或中心凭据。
use super::*;
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "lowercase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Input<'a> {
    Create {
        unit: String,
        #[serde(borrow)]
        snapshot: crate::projection_delivery::sender::snapshot::Wire<'a>,
    },
    Inspect {
        envelope: Envelope,
    },
    Accept {
        envelope: Envelope,
        unit: String,
        revision: u64,
        digest: String,
    },
    Update {
        envelope: Envelope,
        prior_revision: u64,
        revision: u64,
        #[serde(borrow)]
        snapshot: crate::projection_delivery::sender::snapshot::Wire<'a>,
    },
    Read {
        envelope: Envelope,
        revision: u64,
        digest: String,
        receiver_unit: Option<String>,
    },
    Unlink {
        envelope: Envelope,
    },
}

fn path(name: &str) -> PathBuf {
    let value = PathBuf::from(std::env::var_os(name).expect("explicit isolated path required"));
    assert!(value.is_absolute(), "absolute isolated path required");
    value
}

#[test]
#[ignore = "requires the explicitly isolated account-service/Redis/two-Loom fixture"]
fn actual_loom_account_projection_native_driver() {
    let spec = path("OTTER_PROJECTION_ACCOUNT_SPEC");
    let output = path("OTTER_PROJECTION_ACCOUNT_RESULT");
    let profile = path("OTTER_PROJECTION_ACCOUNT_PROFILE");
    let manifest = path("OTTER_PROJECTION_ACCOUNT_MANIFEST");
    assert_eq!(
        spec.parent(),
        output.parent(),
        "spec and output share the owned fixture root"
    );
    let mut bytes = Vec::new();
    File::open(spec)
        .unwrap()
        .take(MAX_WIRE as u64 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= MAX_WIRE);
    let input: Input<'_> = serde_json::from_slice(&bytes).unwrap();
    let discovery = || {
        crate::loom_discovery::Config::from_lookup(|key| {
            (key == std::ffi::OsStr::new("LOOM_MANIFEST_PATH"))
                .then(|| manifest.clone().into_os_string())
        })
    };
    // 每次命令恢复同一个隔离 Otter actor，再由当前 Device 会话建立账号接口。
    let connected =
        crate::loom_local_connection::connect(discovery(), &profile, &Cancel::new()).unwrap();
    let mut owner = crate::device_authorization_owner::Owner::new();
    owner
        .install(&connected.origin, connected.session, now().unwrap())
        .unwrap();
    let lease = owner.lease(&connected.origin, now().unwrap()).unwrap();
    let account = lease.account_projections(&discovery()).unwrap();
    let image = match &input {
        Input::Create { snapshot, .. } | Input::Update { snapshot, .. } => {
            // 借用同一 PNG 字符串，生产解码仍独立验证真实尺寸、摘要和共享像素预算。
            let snapshot = crate::projection_delivery::sender::snapshot::Wire {
                image_base64: snapshot.image_base64,
                width: snapshot.width,
                height: snapshot.height,
            };
            Some(Image(
                crate::projection_delivery::sender::snapshot::Snapshot::decode(
                    snapshot,
                    lease.preparation_cancel(),
                )
                .unwrap(),
            ))
        }
        _ => None,
    };
    let operation = match &input {
        Input::Create { unit, .. } => Operation::Create {
            unit,
            kind: "sticker",
            image: image.as_ref().unwrap(),
        },
        Input::Inspect { envelope } => Operation::Inspect { envelope },
        Input::Accept {
            envelope,
            unit,
            revision,
            digest,
        } => Operation::Accept {
            envelope,
            unit,
            revision: *revision,
            digest,
            confirmed: true,
        },
        Input::Update {
            envelope,
            prior_revision,
            revision,
            ..
        } => Operation::Update {
            envelope,
            prior_revision: *prior_revision,
            revision: *revision,
            image: image.as_ref().unwrap(),
        },
        Input::Read {
            envelope,
            revision,
            digest,
            receiver_unit,
        } => Operation::Read {
            envelope,
            known_revision: *revision,
            known_digest: digest,
            receiver_unit: receiver_unit.as_deref(),
        },
        Input::Unlink { envelope } => Operation::Unlink { envelope },
    };
    let result = match account.execute(operation) {
        Ok(State::Unlinked) => json!({"unlinked":true}),
        Ok(State::Content(content)) => json!({
            "envelope":content.envelope, "revision":content.revision, "digest":content.digest,
            "linked":content.linked, "receiverDeviceId":content.receiver_device_id,
            "receiverUnitId":content.receiver_unit_id, "snapshot":content.image,
            "transport":content.transport, "error":content.error,
        }),
        Err(Error::Remote { status, code }) => json!({"httpStatus":status, "error":code}),
        Err(error) => panic!("native account operation failed: {error:?}"),
    };
    let report = json!({"contextDeviceId":account.context().device_id,
        "otterDeviceId":lease.device_id(), "result":result});
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(bytes.len() <= MAX_WIRE);
    File::options()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap()
        .write_all(&bytes)
        .unwrap();
}
