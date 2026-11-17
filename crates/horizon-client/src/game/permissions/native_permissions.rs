//! Permission nodes for server mods: gathering them and the handler when a server starts, the
//! host's query with its recursion guard, and dropping the service when the server closes.

use super::Game;
use super::handler::gather_handlers;
use super::permission_service::{LevelSource, ModAnswers, PermissionService};
use mod_api::{
    Answered, PermissionContext, PermissionError, PermissionLevel, PermissionNode, PermissionValue,
    PermissionValueType,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// The live operator level (0 to 4) of each online player the game is asking about, by UUID. The
/// built-in handler reads it before the operator list: a dedicated server's level is in that list,
/// an integrated or LAN server has none, and its players' levels exist only on the player.
type LiveLevels = Arc<Mutex<HashMap<u128, u8>>>;

/// Whether a query is being answered, so one asked again meanwhile is refused as recursive.
/// Side: server.
#[derive(Default)]
pub(crate) struct QueryState {
    active: bool,
    tripped: bool,
    live_levels: LiveLevels,
}

impl QueryState {
    /// Starts a query. Refused with `Recursive` while another is being answered, and the
    /// enclosing query is then refused too when it ends (`leave`).
    fn enter(&mut self) -> Result<(), PermissionError> {
        if self.active {
            self.tripped = true;
            return Err(PermissionError::Recursive);
        }
        self.active = true;
        Ok(())
    }

    /// Ends the query `enter` started; a query asked again while it was answered fails it too.
    fn leave<T>(&mut self, result: Result<T, PermissionError>) -> Result<T, PermissionError> {
        self.active = false;
        if std::mem::take(&mut self.tripped) {
            return Err(PermissionError::Recursive);
        }
        result
    }

    fn set_live_level(&self, uuid: u128, level: Option<u8>) {
        if let Ok(mut levels) = self.live_levels.lock() {
            match level {
                Some(level) => levels.insert(uuid, level),
                None => levels.remove(&uuid),
            };
        }
    }
}

/// What a query needs of the game: its service and guard, who is online, and the mods' answers.
/// The game implements it; the tests do with a fake that can ask again from inside an answer.
pub(crate) trait PermissionHost: ModAnswers {
    /// The service and the guard of the open server.
    fn permission_parts(&mut self) -> (&mut PermissionService, &mut QueryState);

    /// The live operator level of `uuid`, or `None` when the player is not on this server.
    fn online_level(&self, uuid: u128) -> Option<u8>;
}

/// Answers `node` for `uuid` through the server's selected handler. `online` is `uuid` when the
/// player is on this server. The service is taken out of the host while it answers, because a mod
/// handler is handed the host: a query it asks again finds the guard set and fails with
/// `Recursive`, and so does the enclosing query. Side: server.
pub(crate) fn run_query<H: PermissionHost>(
    host: &mut H,
    node: &PermissionNode,
    uuid: u128,
    contexts: &[PermissionContext],
) -> Result<PermissionValue, PermissionError> {
    let level = host.online_level(uuid);
    let (service, state) = host.permission_parts();
    state.enter()?;
    state.set_live_level(uuid, level);
    let mut taken = std::mem::take(service);
    let result = taken.query_with(node, level.map(|_| uuid), uuid, contexts, host);
    let (service, state) = host.permission_parts();
    *service = taken;
    state.leave(result)
}

/// `run_query` for a Boolean node. Refused with `WrongType` for any other node, before any handler
/// runs. Side: server.
pub(crate) fn run_query_bool<H: PermissionHost>(
    host: &mut H,
    node: &PermissionNode,
    uuid: u128,
) -> Result<bool, PermissionError> {
    if node.value_type != PermissionValueType::Boolean {
        return Err(PermissionError::WrongType);
    }
    match run_query(host, node, uuid, &[])? {
        PermissionValue::Boolean(answer) => Ok(answer),
        _ => Err(PermissionError::WrongType),
    }
}

impl ModAnswers for Game<'_> {
    fn answer(
        &mut self,
        handler: &str,
        node: &PermissionNode,
        online: Option<u128>,
        uuid: u128,
        contexts: &[PermissionContext],
    ) -> Option<Answered<Result<PermissionValue, PermissionError>>> {
        let mut answer = None;
        self.native_permission_handler_query(handler, node, online, uuid, contexts, &mut answer);
        answer
    }
}

impl PermissionHost for Game<'_> {
    fn permission_parts(&mut self) -> (&mut PermissionService, &mut QueryState) {
        (&mut self.permission_service, &mut self.permission_query_state)
    }

    fn online_level(&self, uuid: u128) -> Option<u8> {
        self.lan.as_ref().and_then(|host| {
            host.hub
                .online_players()
                .find(|guest| guest.uuid == uuid)
                .map(|guest| guest.permission)
        })
    }
}

impl Game<'_> {
    /// Gathers this server's permission nodes and handlers and selects the handler
    /// `configured_handler` names (`permission-handler`; empty selects the built-in one). Asked
    /// once per server start, after the operator list is read and before `server_starting`. The
    /// built-in handler reads an online player's live operator level, else the operator list by
    /// UUID. Side: server.
    pub(super) fn gather_permission_nodes(&mut self, configured_handler: &str) {
        let lists = self.server_lists.clone();
        let live = Arc::clone(&self.permission_query_state.live_levels);
        let levels: LevelSource = Arc::new(move |online, uuid| {
            let live_level = online
                .and_then(|_| live.lock().ok())
                .and_then(|levels| levels.get(&uuid).copied())
                .map(i32::from);
            let level = live_level.or_else(|| {
                let lists = lists.as_ref()?.lock().ok()?;
                lists
                    .ops
                    .entries()
                    .into_iter()
                    .find(|entry| entry.user.id == uuid)
                    .map(|entry| entry.level)
            });
            level.map_or(PermissionLevel::All, PermissionLevel::from_level)
        });
        let mut service = PermissionService::new();
        service.set_level_source(levels);
        Self::native_permission_gather_nodes(&mut service);
        let mut handlers = Vec::new();
        self.native_permission_gather_handler(&mut handlers);
        if let Err(error) = gather_handlers(&mut service, &handlers, configured_handler) {
            log::error!("permission handlers were not gathered ({error}): no permission query is answered");
        }
        self.permission_service = service;
    }

    /// Drops this server's permission service when the server closes; a reopened server gathers
    /// afresh. Side: server.
    pub(super) fn reset_permission_service(&mut self) {
        self.permission_service = PermissionService::new();
        self.permission_query_state = QueryState::default();
    }

    /// Answers `node` for `uuid` through this server's selected handler. Online and offline UUIDs
    /// are answered. Side: server.
    pub(super) fn permission_query(
        &mut self,
        node: &PermissionNode,
        uuid: u128,
        contexts: &[PermissionContext],
    ) -> Result<PermissionValue, PermissionError> {
        run_query(self, node, uuid, contexts)
    }

    /// `permission_query` for a Boolean node. Side: server.
    pub(super) fn permission_query_bool(
        &mut self,
        node: &PermissionNode,
        uuid: u128,
    ) -> Result<bool, PermissionError> {
        run_query_bool(self, node, uuid)
    }
}
