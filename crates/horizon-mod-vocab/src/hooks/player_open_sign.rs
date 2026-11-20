//! The `player_open_sign` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0300

pub trait Hooks {
    /// A server player is about to open a sign for editing, before the editor state is made. `front`
    /// selects the front or back side. Set `cancel` to keep the editor closed. Server-side.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        cancellable = true,
        neoforge = "none (NeoForge has no such event)"
    )]
    fn player_open_sign();
}
