//! Permission handlers the mods provide: their ids as gathered, the selection from
//! `permission-handler`, and the handler that asks the providing mod through
//! `permission_handler_query`. A failed answer is an explicit error, never a grant. Side: server.
//!
//! L1 provides (in `permissions/mod.rs`): `PermissionHandler`, `PermissionService`, the
//! `register_handler_factory` / `finish_gathering` / `query` contract, and the `mod handler;`
//! line that includes this file.

use super::{PermissionHandler, PermissionService};
use mod_api::{
    Answered, PermissionContext, PermissionError, PermissionNode, PermissionValue,
    PermissionValueType,
};
use std::rc::Rc;

/// Asks the mods for one query of a handler they provide. The answer names the mod that gave
/// it. `None` means no mod answered. The game implements it by firing `permission_handler_query`.
/// Side: server.
pub(crate) trait ModAnswers {
    /// Asks for `node` of handler `handler`, for the online player `online` (`None` when the
    /// UUID is offline) with the declared `contexts`.
    fn answer(
        &self,
        handler: &str,
        node: &PermissionNode,
        online: Option<u128>,
        uuid: u128,
        contexts: &[PermissionContext],
    ) -> Option<Answered<Result<PermissionValue, PermissionError>>>;
}

/// A handler a mod provided. Every query goes to that mod. Side: server.
pub(crate) struct ModPermissionHandler {
    id: String,
    owner: String,
    answers: Rc<dyn ModAnswers>,
}

impl PermissionHandler for ModPermissionHandler {
    fn id(&self) -> &str {
        &self.id
    }

    /// Refuses without a grant: no answer, an answer from another mod, a value of the wrong type
    /// and a failed answer are all errors. Side: server.
    fn query(
        &mut self,
        node: &PermissionNode,
        online: Option<u128>,
        uuid: u128,
        contexts: &[PermissionContext],
    ) -> Result<PermissionValue, PermissionError> {
        let Some(answered) = self.answers.answer(&self.id, node, online, uuid, contexts) else {
            log::warn!(
                "permission node \"{}\": handler \"{}\" got no answer from mod \"{}\"",
                node.name,
                self.id,
                self.owner
            );
            return Err(PermissionError::HandlerFailed);
        };
        if answered.mod_id != self.owner.as_str() {
            log::warn!(
                "permission node \"{}\": handler \"{}\" was answered by mod \"{}\", not by its provider \"{}\"",
                node.name,
                self.id,
                answered.mod_id,
                self.owner
            );
            return Err(PermissionError::HandlerFailed);
        }
        match answered.answer {
            Ok(value) if value_matches(&value, &node.value_type) => Ok(value),
            Ok(_) => {
                log::warn!(
                    "permission node \"{}\": handler \"{}\" answered a value of another type",
                    node.name,
                    self.id
                );
                Err(PermissionError::WrongType)
            }
            Err(error) => {
                log::warn!(
                    "permission node \"{}\": handler \"{}\" failed in mod \"{}\"",
                    node.name,
                    self.id,
                    self.owner
                );
                Err(error)
            }
        }
    }
}

/// Whether `value` has the declared type `ty`. Side: server.
fn value_matches(value: &PermissionValue, ty: &PermissionValueType) -> bool {
    matches!(
        (value, ty),
        (PermissionValue::Boolean(_), PermissionValueType::Boolean)
            | (PermissionValue::Integer(_), PermissionValueType::Integer)
            | (PermissionValue::String(_), PermissionValueType::String)
            | (PermissionValue::Text(_), PermissionValueType::Text)
    )
}

/// The handler `permission-handler` selects. An empty setting selects none. A name no mod
/// gathered is logged, naming the id, and selects none: the default handler answers. Side: server.
pub(crate) fn configured_handler(configured: &str, gathered: &[Answered<String>]) -> Option<String> {
    if configured.is_empty() {
        return None;
    }
    if gathered.iter().any(|handler| handler.answer == configured) {
        return Some(configured.to_owned());
    }
    log::warn!(
        "permission-handler \"{configured}\" names no gathered handler; the default handler answers permission queries"
    );
    None
}

/// Registers each gathered handler factory, selects one and publishes the service through
/// `finish_gathering`. A duplicate or empty id returns its error before anything is selected, so
/// the service stays uninitialized and refuses every query. The node set must already be gathered
/// (L1). Side: server.
pub(crate) fn gather_handlers(
    service: &mut PermissionService,
    gathered: &[Answered<String>],
    answers: &Rc<dyn ModAnswers>,
    configured: &str,
) -> Result<(), PermissionError> {
    for entry in gathered {
        if entry.answer.is_empty() {
            return Err(PermissionError::InvalidInput);
        }
        let id = entry.answer.clone();
        let owner = entry.mod_id.to_owned();
        let answers = Rc::clone(answers);
        let handler_id = id.clone();
        service.register_handler_factory(
            &id,
            Box::new(move |_nodes: &[PermissionNode]| -> Box<dyn PermissionHandler> {
                Box::new(ModPermissionHandler {
                    id: handler_id,
                    owner,
                    answers,
                })
            }),
        )?;
    }
    let selected = configured_handler(configured, gathered);
    service.finish_gathering(selected.as_deref());
    Ok(())
}

#[cfg(test)]
#[path = "handler_tests.rs"]
mod tests;
