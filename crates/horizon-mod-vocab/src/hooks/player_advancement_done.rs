//! The `player_advancement_done` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0275

pub trait Hooks {
    /// A server player completed an advancement, after progress was recorded. Set `cancel` to
    /// withhold its toast, announcement and rewards; the advancement stays done in the progress.
    /// Server-side. Not the message override (Paper's `message(Component)`): only the cancel.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        neoforge = "AdvancementEvent.AdvancementEarnEvent",
        cancellable = true
    )]
    fn player_advancement_done();
}
