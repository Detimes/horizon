//! The `inventory_move_item` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0228

pub trait Hooks {
    /// A hopper is about to move one item between containers, a composter's
    /// bone meal included. Set `cancel` to leave it in its source slot. A chest
    /// boat or minecart is reported at the block it is in. `source_slot` is the
    /// slot the item leaves in the source container's numbering (a composter's
    /// bone meal is its slot 0; a capability's slot is the capability's), and
    /// `amount` how many leave: a hopper moves one item per transfer.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        host = "read",
        neoforge = "none (NeoForge has no such event; Bukkit's InventoryMoveItemEvent)",
        cancellable = true,
        version = 2
    )]
    fn inventory_move_item();
}
