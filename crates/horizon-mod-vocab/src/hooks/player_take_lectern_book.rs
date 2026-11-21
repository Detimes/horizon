//! The `player_take_lectern_book` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0263

pub trait Hooks {
    /// A server player is about to take the book from a lectern, before the menu and block entity
    /// change. `book` is the item id of the book it takes. Set `cancel` to leave the book on the
    /// lectern. Server-side.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        cancellable = true,
        neoforge = "none (NeoForge has no such event)"
    )]
    fn player_take_lectern_book();
}
