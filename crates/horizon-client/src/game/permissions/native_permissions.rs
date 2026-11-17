//! Permission nodes for server mods: gathering them when a server starts, the host's query, and
//! dropping the service when the server closes.

use super::Game;
use super::permission_service::{LevelSource, PermissionService};
use mod_api::{
    PermissionContext, PermissionError, PermissionLevel, PermissionNode, PermissionValue,
};
use std::sync::Arc;

impl Game<'_> {
    /// Gathers this server's permission nodes and selects its handler. Asked once per server start,
    /// after the operator list is read and before `server_starting`. The built-in handler reads
    /// operator levels from the operator list, by UUID. Side: server.
    pub(super) fn gather_permission_nodes(&mut self) {
        let lists = self.server_lists.clone();
        let levels: LevelSource = Arc::new(move |_online, uuid| {
            let listed = lists.as_ref().and_then(|lists| {
                let lists = lists.lock().ok()?;
                let level = lists
                    .ops
                    .entries()
                    .into_iter()
                    .find(|entry| entry.user.id == uuid)
                    .map(|entry| entry.level);
                level
            });
            listed.map_or(PermissionLevel::All, PermissionLevel::from_level)
        });
        let mut service = PermissionService::new();
        service.set_level_source(levels);
        Self::native_permission_gather_nodes(&mut service);
        // The configured handler (`permission-handler`) is passed here once the server reads it.
        service.finish_gathering(None);
        self.permission_service = service;
    }

    /// Drops this server's permission service when the server closes; a reopened server gathers
    /// afresh. Side: server.
    pub(super) fn reset_permission_service(&mut self) {
        self.permission_service = PermissionService::new();
    }

    /// Answers `node` for `uuid` through this server's selected handler. An online player is passed
    /// as their UUID; an offline UUID is answered as well. Side: server.
    pub(super) fn permission_query(
        &mut self,
        node: &PermissionNode,
        uuid: u128,
        contexts: &[PermissionContext],
    ) -> Result<PermissionValue, PermissionError> {
        let online = self.lan.as_ref().and_then(|host| {
            host.hub
                .online_players()
                .any(|guest| guest.uuid == uuid)
                .then_some(uuid)
        });
        self.permission_service.query(node, online, uuid, contexts)
    }
}
