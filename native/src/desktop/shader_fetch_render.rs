//! Authenticated, fully released official code-fetch delivery -> local derived results, always completed on the original execution worker.
use super::{
    loom_execution_session::Final, loom_input_job::PreparedBatch, shader_prepare::Runtime,
};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub enum Error {
    Delivery(super::loom_delivery::TakeError),
    Prepare(super::shader_payload::Error),
    Render(super::shader_invocation::Failure),
    Image(windows::core::Error),
    Output(super::loom_output_batch::Error),
    Cancelled,
}
pub(super) fn render(
    final_result: &mut Final,
    inputs: &PreparedBatch,
    runtime: Runtime,
    cancel: &AtomicBool,
) -> Result<Option<super::shader_recipe_cache::Acquired>, Error> {
    check(cancel)?;
    let batch = final_result
        .delivery
        .take_released()
        .map_err(Error::Delivery)?;
    let (invocation, recipe) = runtime
        .acquire(batch, inputs, cancel)
        .map_err(Error::Prepare)?;
    let frame = invocation
        .render(&|| cancel.load(Ordering::Acquire))
        .map_err(Error::Render)?;
    drop(invocation);
    let batch = super::shader_image::prepare(frame, cancel)
        .map_err(Error::Image)?
        .into_batch()
        .map_err(Error::Output)?;
    check(cancel)?;
    final_result
        .delivery
        .install_derived(batch)
        .map_err(Error::Delivery)?;
    // Remote candidates belong to the code-fetch response; unrendered candidates are not candidates for the local official PNG.
    final_result.candidates = None;
    Ok(recipe)
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
