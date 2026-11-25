//! Save local operations with response expectations; no arbitrary paths, central operations, account credentials, or endpoint parameters.
use super::*;
use zeroize::Zeroizing;
pub enum Operation<'a> {
    Create {
        unit: &'a str,
        kind: &'a str,
        image: &'a Image,
    },
    Inspect {
        envelope: &'a Envelope,
    },
    Accept {
        envelope: &'a Envelope,
        unit: &'a str,
        revision: u64,
        digest: &'a str,
        confirmed: bool,
    },
    Update {
        envelope: &'a Envelope,
        prior_revision: u64,
        revision: u64,
        image: &'a Image,
    },
    /// receiver_unit=None means local source; receivers retain the exact Unit/original invitation, never rebind from network replies.
    Read {
        envelope: &'a Envelope,
        known_revision: u64,
        known_digest: &'a str,
        receiver_unit: Option<&'a str>,
    },
    Unlink {
        envelope: &'a Envelope,
    },
}
impl Operation<'_> {
    pub(super) fn route(&self) -> &'static str {
        match self {
            Self::Create { .. } => "create",
            Self::Inspect { .. } => "inspect",
            Self::Accept { .. } => "accept",
            Self::Update { .. } => "update",
            Self::Read { .. } => "read",
            Self::Unlink { .. } => "unlink",
        }
    }
    pub(super) fn envelope(&self) -> Option<&Envelope> {
        match self {
            Self::Create { .. } => None,
            Self::Inspect { envelope }
            | Self::Accept { envelope, .. }
            | Self::Update { envelope, .. }
            | Self::Read { envelope, .. }
            | Self::Unlink { envelope } => Some(envelope),
        }
    }
    pub(super) fn validate(&self, context: &Context) -> Result<(), Error> {
        if let Some(envelope) = self.envelope() {
            envelope.validate()?;
            if !envelope.matches_origin(context) {
                return Err(Error::Identity);
            }
        }
        let valid = match self {
            Self::Create { unit, kind, .. } => {
                identifier(unit) && matches!(*kind, "sticker" | "art")
            }
            Self::Inspect { envelope } => envelope.source_device() != context.device_id,
            Self::Accept {
                envelope,
                unit,
                revision: rev,
                digest: hash,
                confirmed,
            } => {
                *confirmed
                    && identifier(unit)
                    && revision(*rev)
                    && digest(hash)
                    && envelope.source_device() != context.device_id
            }
            Self::Update {
                envelope,
                prior_revision,
                revision: rev,
                ..
            } => {
                envelope.matches_source(context)
                    && revision(*prior_revision)
                    && revision(*rev)
                    && prior_revision.checked_add(1) == Some(*rev)
            }
            Self::Read {
                envelope,
                known_revision,
                known_digest,
                receiver_unit,
            } => {
                revision(*known_revision)
                    && digest(known_digest)
                    && receiver_unit.map_or(envelope.matches_source(context), |unit| {
                        identifier(unit) && envelope.source_device() != context.device_id
                    })
            }
            Self::Unlink { .. } => true,
        };
        if valid {
            Ok(())
        } else {
            Err(Error::Input)
        }
    }
    pub(super) fn body(&self) -> Result<Zeroizing<Vec<u8>>, Error> {
        // Serialize borrowed image/envelope; never clone an entire Base64 PNG through Value.
        #[derive(Serialize)]
        #[serde(
            tag = "kind",
            rename_all = "lowercase",
            rename_all_fields = "camelCase"
        )]
        enum Body<'a> {
            Create {
                unit_id: &'a str,
                content_kind: &'a str,
                snapshot: &'a Image,
            },
            Inspect {
                envelope: &'a Envelope,
            },
            Accept {
                envelope: &'a Envelope,
                receiver_unit_id: &'a str,
                expected_revision: u64,
                expected_digest: &'a str,
                confirmed: bool,
            },
            Update {
                projection_id: &'a str,
                source_session_id: &'a str,
                prior_revision: u64,
                revision: u64,
                snapshot: &'a Image,
            },
            Read {
                projection_id: &'a str,
                known_revision: u64,
            },
            Unlink {
                projection_id: &'a str,
            },
        }
        let body = match self {
            Self::Create { unit, kind, image } => Body::Create {
                unit_id: unit,
                content_kind: kind,
                snapshot: image,
            },
            Self::Inspect { envelope } => Body::Inspect { envelope },
            Self::Accept {
                envelope,
                unit,
                revision,
                digest,
                confirmed,
            } => Body::Accept {
                envelope,
                receiver_unit_id: unit,
                expected_revision: *revision,
                expected_digest: digest,
                confirmed: *confirmed,
            },
            Self::Update {
                envelope,
                prior_revision,
                revision,
                image,
            } => Body::Update {
                projection_id: envelope.projection_id(),
                source_session_id: envelope.source_session(),
                prior_revision: *prior_revision,
                revision: *revision,
                snapshot: image,
            },
            Self::Read {
                envelope,
                known_revision,
                ..
            } => Body::Read {
                projection_id: envelope.projection_id(),
                known_revision: *known_revision,
            },
            Self::Unlink { envelope } => Body::Unlink {
                projection_id: envelope.projection_id(),
            },
        };
        serde_json::to_vec(&body)
            .map(Zeroizing::new)
            .map_err(|_| Error::Input)
    }
}
