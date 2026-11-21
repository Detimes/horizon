//! The `prepare_hopper_transfer` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0323

pub trait Hooks {
    /// A hopper at `pos` began an item transfer attempt. This runs after its
    /// cooldown and enabled checks, before pushing or pulling any items. Set
    /// `cancel` to skip this attempt: nothing is pushed or pulled. Server-side,
    /// including the single-player server.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        cancellable = true,
        neoforge = "none (NeoForge has no hopper event)"
    )]
    fn prepare_hopper_transfer();
}
