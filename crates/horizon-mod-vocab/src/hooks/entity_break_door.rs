//! The `entity_break_door` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0198

pub trait Hooks {
    /// A mob is about to break the door at `pos` (Paper `EntityBreakDoorEvent`,
    /// as `BreakDoorGoal.tick` removes it). Set `cancel` and the door stays
    /// as it is: no block is removed and no level event plays. Server-side.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        cancellable = true,
        neoforge = "none (NeoForge has no such event)"
    )]
    fn entity_break_door();
}
