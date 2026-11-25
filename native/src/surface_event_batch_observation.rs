//! Bounded observation of batch ACKs: retain each result and output, but only one scene at the highest revision.
use crate::{
    live_relay_http::{BatchSubmission, Control, ConvergenceError},
    surface_click_event::ClickEvent,
    surface_convergence::Snapshot,
    surface_result_commit::Commit,
};
use std::time::{Duration, Instant};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unavailable(super::Error),
    Panicked,
    Convergence(ConvergenceError),
    Confirmation(crate::live_relay_http::ConfirmationError),
    Decision(crate::surface_decision_runtime::Error),
}
pub struct Item {
    pub decision: Option<crate::surface_decision_runtime::Evidence>,
    /// Corresponds to the same-index event in submission.acknowledgements; excludes the unsubmitted suffix.
    pub result: Result<u64, Error>,
    pub commit: Option<Commit>,
    pub confirmation: Option<Result<crate::surface_confirmation::Request, Error>>,
}
pub struct Observation {
    pub items: Vec<Item>,
    /// Only a candidate snapshot; deliver to the window only after checking all relevant action/authorization conditions.
    pub snapshot: Option<Snapshot>,
}
pub(super) fn observe(
    control: &Control<'_>,
    events: &[ClickEvent],
    submission: &mut BatchSubmission,
) -> Result<Observation, super::Error> {
    let mut observation = Observation {
        items: Vec::new(),
        snapshot: None,
    };
    observation
        .items
        .try_reserve_exact(submission.acknowledgements.len())
        .map_err(|_| super::Error::Allocation)?;
    let deadline = Instant::now() + Duration::from_secs(25);
    for (event, ack) in events.iter().zip(&mut submission.acknowledgements) {
        let observed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            control.converge_surface(event, ack, deadline)
        }));
        let mut item = match observed {
            Err(_) => Item {
                decision: None,
                result: Err(Error::Panicked),
                commit: None,
                confirmation: None,
            },
            Ok(outcome) => {
                *ack = outcome.ack;
                match outcome.result {
                    Err(error) => Item {
                        decision: None,
                        result: Err(Error::Convergence(error)),
                        commit: None,
                        confirmation: None,
                    },
                    Ok(mut snapshot) => {
                        let item = Item {
                            decision: None,
                            result: Ok(snapshot.revision),
                            commit: snapshot.commit.take(),
                            confirmation: None,
                        };
                        if observation
                            .snapshot
                            .as_ref()
                            .is_none_or(|current| current.revision <= snapshot.revision)
                        {
                            observation.snapshot = Some(snapshot);
                        }
                        item
                    }
                }
            }
        };
        if item.result == Err(Error::Convergence(ConvergenceError::ConfirmationRequired)) {
            let prompt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                control.surface_confirmation_until(event, ack, deadline)
            }));
            item.confirmation = Some(match prompt {
                Ok(outcome) => {
                    *ack = outcome.ack;
                    outcome.result.map_err(Error::Confirmation)
                }
                Err(_) => Err(Error::Panicked),
            });
        }
        observation.items.push(item);
    }
    Ok(observation)
}
