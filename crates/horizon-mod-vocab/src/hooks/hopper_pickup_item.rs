//! The `hopper_pickup_item` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0225

pub trait Hooks {
    /// A hopper at `pos` is about to pick up `count` of `item`. `entity` is the dropped item's
    /// entity ID, so a mod can find the drop it is about (a locked death drop, for one). Set `cancel` to leave the dropped stack in the world. Server-side.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        neoforge = "none (NeoForge has no direct equivalent; Bukkit's InventoryPickupItemEvent)",
        cancellable = true,
        version = 2,
        target = "horizon_client::game::native_hooks::base::GameLevel<'_>::native_hopper_pickup_item"
    )]
    fn hopper_pickup_item();
}
