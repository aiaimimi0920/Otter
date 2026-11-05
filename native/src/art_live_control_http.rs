//! Native menu-only controller operations; Preview is host evidence, not a JS input API.
use super::*;
use crate::{
    live_input::{Event, MAX_SEQUENCE},
    live_relay_http::{Control, Error, RequesterCursor},
};
use serde_json::{json, Value};
const CONTROL: &str = "loom.art.local.live-control.v1";
impl Control<'_> {
    fn local_art_operation(
        &self,
        proof: &Proof,
        operation: Value,
        reducing: bool,
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, Error> {
        if !reducing {
            proof.validate()?;
        }
        if proof.binding.origin != self.client.origin()
            || proof.binding.device_id != self.session.device_id()
        {
            return Err(Error::Input);
        }
        let bytes = serde_json::to_vec(&json!({"protocolVersion":CONTROL,"binding":proof.binding,"grant":proof.envelope,"operation":operation})).map_err(|_| Error::Input)?;
        if bytes.len() > 32 * 1024 {
            return Err(Error::Input);
        }
        let response = self.send(
            "POST",
            "/v1/art/local/live-viewer/control",
            &bytes,
            Duration::from_secs(3),
        )?;
        if !reducing {
            proof.validate()?;
        }
        Ok(response)
    }
    pub(crate) fn change_local_art_controller(
        &self,
        identity: &Identity,
        proof: &Proof,
        sequence: u64,
        acquire: bool,
    ) -> Result<(), Error> {
        if sequence == 0 || sequence > MAX_SEQUENCE {
            return Err(Error::Cursor);
        }
        let bytes = self.local_art_operation(proof, json!({"kind":if acquire {"acquire"}else{"release"},"sessionId":identity.session(),"epoch":identity.epoch(),"sequence":sequence}), !acquire)?;
        self.validate_controller_reply(identity, acquire, &bytes)
    }
    pub(crate) fn forward_local_art_input(
        &self,
        identity: &Identity,
        proof: &Proof,
        cursor: RequesterCursor,
        event: Event,
    ) -> Result<u64, Error> {
        let bytes = self.local_art_operation(
            proof,
            json!({"kind":"input","envelope":self.window_input_envelope(identity, cursor, event)?}),
            false,
        )?;
        Self::validate_input_reply(&bytes)
    }
}
