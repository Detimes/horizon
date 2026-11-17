//! The `permission_handler_query` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The selected permission handler of this server is asked to answer one node query. Only the
    /// mod that provided the handler `handler` (through `permission_gather_handler`) answers it;
    /// other mods leave it unanswered. Answer with the node's typed value, or with the explicit
    /// error the handler chooses. The game refuses an answer from a mod other than the one that
    /// provided the handler, and treats an unanswered query as a handler failure: the query never
    /// falls back to a grant. `online` is the online player's handle, `None` for an offline UUID,
    /// and `contexts` holds only the context keys the node declares. Side: server.
    #[hook(
        side = "server",
        contract = "replace",
        out = "answer",
        answered_by = "mod",
        neoforge = "none (no NeoForge counterpart; permission nodes are a Horizon service)"
    )]
    fn permission_handler_query();
}
