//! Permission handlers the mods provide: their ids as gathered, the selection from
//! `permission-handler`, and the handler that asks the providing mod through
//! `permission_handler_query`. A failed answer is an explicit error, never a grant. Side: server.
//!
//! The permission service (`permission_service.rs`) provides `PermissionHandler`,
//! `register_handler_factory`, `finish_gathering` and `query`.

use super::permission_service::{ModAnswers, PermissionHandler, PermissionService};
use mod_api::{
    PermissionContext, PermissionError, PermissionNode, PermissionValue, PermissionValueType,
};

/// A handler a mod provided. Every query goes to that mod. Side: server.
pub(crate) struct ModPermissionHandler {
    id: String,
    owner: String,
}

impl ModPermissionHandler {
    /// The handler `id` that the mod `owner` provides.
    pub(crate) fn new(id: &str, owner: &str) -> Self {
        Self {
            id: id.to_owned(),
            owner: owner.to_owned(),
        }
    }
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
        answers: &mut dyn ModAnswers,
    ) -> Result<PermissionValue, PermissionError> {
        let Some(answered) = answers.answer(&self.id, node, online, uuid, contexts) else {
            log::warn!(
                "permission node \"{}\": handler \"{}\" got no answer from mod \"{}\"",
                node.name,
                self.id,
                self.owner
            );
            return Err(PermissionError::HandlerFailed);
        };
        if answered.mod_id != self.owner {
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
pub(crate) fn configured_handler(configured: &str, gathered: &[(&str, &str)]) -> Option<String> {
    if configured.is_empty() {
        return None;
    }
    if gathered.iter().any(|(id, _)| *id == configured) {
        return Some(configured.to_owned());
    }
    log::warn!(
        "permission-handler \"{configured}\" names no gathered handler; the default handler answers permission queries"
    );
    None
}

/// Registers each gathered handler factory (`(id, providing mod)` pairs), selects one and publishes the service through
/// `finish_gathering`. A duplicate or empty id returns its error before anything is selected, so
/// the service stays uninitialized and refuses every query. The node set must already be gathered.
/// Side: server.
pub(crate) fn gather_handlers(
    service: &mut PermissionService,
    gathered: &[(&str, &str)],
    configured: &str,
) -> Result<(), PermissionError> {
    for (id, owner) in gathered {
        if id.is_empty() {
            return Err(PermissionError::InvalidInput);
        }
        let handler = ModPermissionHandler::new(id, owner);
        service.register_handler_factory(
            id,
            Box::new(
                move |_nodes: &[PermissionNode]| -> Box<dyn PermissionHandler> {
                    Box::new(handler)
                },
            ),
        )?;
    }
    let selected = configured_handler(configured, gathered);
    service.finish_gathering(selected.as_deref());
    Ok(())
}
