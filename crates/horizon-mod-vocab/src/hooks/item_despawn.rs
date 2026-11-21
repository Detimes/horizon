//! The `item_despawn` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0230

pub trait Hooks {
    /// A dropped item entity reached its 6000-tick lifetime and is about to be
    /// removed (Bukkit's `ItemDespawnEvent`). Set `cancel` to keep the item for
    /// 5 more minutes, as Paper's cancel does; a positive `extra_life` from the
    /// `item_expire` hook already keeps it and this hook does not run then.
    /// Server-side.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        neoforge = "none (NeoForge has no such event; Bukkit's ItemDespawnEvent)",
        cancellable = true
    )]
    fn item_despawn();
}
