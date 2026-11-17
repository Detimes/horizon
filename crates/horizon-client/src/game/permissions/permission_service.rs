//! The permission service: typed node declaration, handler selection and queries. It lives
//! with one open server and answers only server-side; every `PermissionError` it returns is an
//! explicit refusal, and no refusal falls back to a grant.

use mod_api::{
    NodeDefault, PermissionContext, PermissionError, PermissionLevel, PermissionNode,
    PermissionNodes, PermissionValue, PermissionValueType, check_contexts, check_node_name,
};
use std::{
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

/// The id of the built-in handler, which answers every node from its default. No mod can gather a
/// handler under this id.
pub(crate) const DEFAULT_HANDLER_ID: &str = "default";

/// The longest accepted handler id, in bytes.
const MAX_HANDLER_ID_LEN: usize = 128;

/// The operator level of a player, asked by UUID. `online` is the UUID when the player is on this
/// server. The game supplies it when the server starts.
pub(crate) type LevelSource = Arc<dyn Fn(Option<u128>, u128) -> PermissionLevel + Send + Sync>;

/// Builds a handler from the complete, validated set of declared nodes.
pub(crate) type HandlerFactory =
    Box<dyn FnOnce(&[PermissionNode]) -> Box<dyn PermissionHandler> + Send>;

/// The one policy that answers every node of a server. It is `Send` because the service that
/// holds it lives in the game.
pub(crate) trait PermissionHandler: Send {
    /// The handler's id, as the server configuration names it.
    fn id(&self) -> &str;

    /// Answers `node` for `uuid` (and `online` when the player is on this server). Returns the
    /// node's declared type or an explicit error. Side: server.
    fn query(
        &mut self,
        node: &PermissionNode,
        online: Option<u128>,
        uuid: u128,
        contexts: &[PermissionContext],
    ) -> Result<PermissionValue, PermissionError>;
}

type NodeKey = (String, PermissionValueType);

/// Declared nodes, gathered handler factories and the selected handler of one server. A server
/// owns one, created when it starts and dropped when it closes.
pub(crate) struct PermissionService {
    nodes: BTreeMap<NodeKey, (PermissionNode, NodeDefault)>,
    factories: BTreeMap<String, HandlerFactory>,
    levels: Option<LevelSource>,
    handler: Option<Box<dyn PermissionHandler>>,
    custom_handler: bool,
    in_query: bool,
}

impl Default for PermissionService {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionService {
    /// An empty service that answers nothing until its gathering finishes.
    pub(crate) fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            factories: BTreeMap::new(),
            levels: None,
            handler: None,
            custom_handler: false,
            in_query: false,
        }
    }

    /// Whether gathering has finished, so queries are answered.
    pub(crate) fn is_initialized(&self) -> bool {
        self.handler.is_some()
    }

    /// Marks a query as being answered, so a nested one is refused as recursive.
    #[cfg(test)]
    pub(crate) fn set_query_in_progress(&mut self, in_progress: bool) {
        self.in_query = in_progress;
    }

    /// The operator level source the built-in handler reads. Set it before `finish_gathering`.
    pub(crate) fn set_level_source(&mut self, levels: LevelSource) {
        self.levels = Some(levels);
    }

    /// Declares `node` with `default`. Refused after gathering has finished, for a bad name, for
    /// a default of the wrong shape, and for a node already declared with the same name and type.
    /// Side: server.
    pub(crate) fn register_node(
        &mut self,
        node: PermissionNode,
        default: NodeDefault,
    ) -> Result<(), PermissionError> {
        if self.is_initialized() {
            return Err(PermissionError::InvalidInput);
        }
        check_node_name(&node.name)?;
        check_default(node.value_type, &default)?;
        let key = (node.name.clone(), node.value_type);
        if self.nodes.contains_key(&key) {
            return Err(PermissionError::DuplicateNode);
        }
        self.nodes.insert(key, (node, default));
        Ok(())
    }

    /// Gathers a handler factory under `id`. Refused after gathering has finished, for an empty or
    /// over-long id, and for an id already gathered or reserved for the built-in handler.
    /// Side: server.
    #[allow(
        dead_code,
        reason = "the handler selection layer gathers its factories through this"
    )]
    pub(crate) fn register_handler_factory(
        &mut self,
        id: &str,
        factory: HandlerFactory,
    ) -> Result<(), PermissionError> {
        if self.is_initialized() {
            return Err(PermissionError::InvalidInput);
        }
        if id.is_empty() || id.len() > MAX_HANDLER_ID_LEN {
            return Err(PermissionError::InvalidInput);
        }
        if id == DEFAULT_HANDLER_ID || self.factories.contains_key(id) {
            return Err(PermissionError::DuplicateHandler);
        }
        self.factories.insert(id.to_owned(), factory);
        Ok(())
    }

    /// Ends gathering: selects the configured factory, or the built-in handler when none is
    /// configured or the configured id is unknown (logged by name), builds the handler from the
    /// declared nodes and starts answering. A second call does nothing.
    pub(crate) fn finish_gathering(&mut self, configured_handler: Option<&str>) {
        if self.is_initialized() {
            return;
        }
        let nodes: Vec<PermissionNode> = self.nodes.values().map(|(n, _)| n.clone()).collect();
        let selected = match configured_handler {
            None | Some(DEFAULT_HANDLER_ID) => None,
            Some(id) => match self.factories.remove(id) {
                Some(factory) => Some(factory),
                None => {
                    log::warn!(
                        "permission-handler \"{id}\" is not gathered: the built-in default handler answers"
                    );
                    None
                }
            },
        };
        self.factories.clear();
        let selected_custom = selected.is_some();
        let handler: Box<dyn PermissionHandler> = match selected {
            Some(factory) => factory(&nodes),
            None => Box::new(DefaultHandler {
                defaults: self
                    .nodes
                    .iter()
                    .map(|(key, (_, default))| (key.clone(), default.clone()))
                    .collect(),
                levels: self.levels.clone(),
            }),
        };
        self.custom_handler = selected_custom;
        self.handler = Some(handler);
    }

    /// Whether a handler other than the built-in one was selected. Side: server.
    pub(crate) fn custom_handler_selected(&self) -> bool {
        self.custom_handler
    }

    /// Answers `node` for `uuid`. `online` is the UUID when the player is on this server; an
    /// offline UUID is answered too. Refused before initialization, while a query is already being
    /// answered, for unregistered nodes (before the handler runs), for bad contexts, and for an
    /// answer of another type than the node's. A handler's failure or panic is `HandlerFailed`.
    /// Side: server.
    pub(crate) fn query(
        &mut self,
        node: &PermissionNode,
        online: Option<u128>,
        uuid: u128,
        contexts: &[PermissionContext],
    ) -> Result<PermissionValue, PermissionError> {
        if !self.is_initialized() {
            return Err(PermissionError::NotInitialized);
        }
        if self.in_query {
            return Err(PermissionError::Recursive);
        }
        check_contexts(contexts)?;
        if !self
            .nodes
            .contains_key(&(node.name.clone(), node.value_type))
        {
            return Err(PermissionError::Unregistered);
        }
        let Some(handler) = self.handler.as_mut() else {
            return Err(PermissionError::NotInitialized);
        };
        self.in_query = true;
        let answer = catch_unwind(AssertUnwindSafe(|| {
            handler.query(node, online, uuid, contexts)
        }))
        .unwrap_or(Err(PermissionError::HandlerFailed));
        self.in_query = false;
        let value = match answer {
            Ok(value) => value,
            Err(error) => {
                log::warn!(
                    "permission node {} (handler {}): {error}",
                    node.name,
                    handler.id()
                );
                return Err(error);
            }
        };
        if value.value_type() != node.value_type {
            log::warn!(
                "permission node {} (handler {}): answered with another value type",
                node.name,
                handler.id()
            );
            return Err(PermissionError::WrongType);
        }
        Ok(value)
    }

    /// `query` for a node that answers Boolean. Refused with `WrongType` for any other node, before
    /// the handler runs. Side: server.
    #[allow(
        dead_code,
        reason = "the command layer asks Boolean nodes through this"
    )]
    pub(crate) fn query_bool(
        &mut self,
        node: &PermissionNode,
        online: Option<u128>,
        uuid: u128,
    ) -> Result<bool, PermissionError> {
        if node.value_type != PermissionValueType::Boolean {
            return Err(PermissionError::WrongType);
        }
        match self.query(node, online, uuid, &[])? {
            PermissionValue::Boolean(answer) => Ok(answer),
            _ => Err(PermissionError::WrongType),
        }
    }
}

impl PermissionNodes for PermissionService {
    fn add(&mut self, node: PermissionNode, default: NodeDefault) -> Result<(), PermissionError> {
        self.register_node(node, default)
    }
}

/// A Boolean node takes a level default, any other node a value of its own type.
fn check_default(
    value_type: PermissionValueType,
    default: &NodeDefault,
) -> Result<(), PermissionError> {
    let fits = match (value_type, default) {
        (PermissionValueType::Boolean, NodeDefault::Level(_)) => true,
        (PermissionValueType::Boolean, NodeDefault::Value(_)) => false,
        (_, NodeDefault::Level(_)) => false,
        (expected, NodeDefault::Value(value)) => value.value_type() == expected,
    };
    if fits {
        Ok(())
    } else {
        Err(PermissionError::WrongType)
    }
}

/// The built-in handler: a Boolean node is granted when the player's operator level is at least
/// its default level; any other node answers its default value. Nothing here reads the world.
struct DefaultHandler {
    defaults: BTreeMap<NodeKey, NodeDefault>,
    levels: Option<LevelSource>,
}

impl PermissionHandler for DefaultHandler {
    fn id(&self) -> &str {
        DEFAULT_HANDLER_ID
    }

    fn query(
        &mut self,
        node: &PermissionNode,
        online: Option<u128>,
        uuid: u128,
        _contexts: &[PermissionContext],
    ) -> Result<PermissionValue, PermissionError> {
        let default = self
            .defaults
            .get(&(node.name.clone(), node.value_type))
            .ok_or(PermissionError::Unregistered)?;
        Ok(match default {
            NodeDefault::Level(required) => {
                let level = self
                    .levels
                    .as_ref()
                    .map_or(PermissionLevel::All, |levels| levels(online, uuid));
                PermissionValue::Boolean(level >= *required)
            }
            NodeDefault::Value(value) => value.clone(),
        })
    }
}
