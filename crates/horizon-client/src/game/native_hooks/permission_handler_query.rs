use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// The selected permission handler is asked to answer one node query; only the mod that
    /// provided `handler` answers. The first mod to answer ends the query. Side: server.
    pub(in crate::game) fn native_permission_handler_query(
        &self,
        handler: &str,
        node: &mod_api::PermissionNode,
        online: Option<u128>,
        uuid: u128,
        contexts: &[mod_api::PermissionContext],
        answer: &mut Option<
            mod_api::Answered<Result<mod_api::PermissionValue, mod_api::PermissionError>>,
        >,
    ) {
        self.fire_permission_handler_query(handler, node, online, uuid, contexts, answer);
    }
}
