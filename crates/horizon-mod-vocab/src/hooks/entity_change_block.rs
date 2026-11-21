//! The `entity_change_block` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0200

pub trait Hooks {
    /// A mob changed a block at `pos` of the level `dimension` (a `&str`,
    /// `minecraft:overworld` and so on), after the new state was applied.
    /// `entity` and `kind` are the mob; `from` and `to` are the block names.
    #[hook(
        side = "server",
        contract = "listen",
        neoforge = "none (NeoForge has no such event)"
    )]
    fn entity_change_block(
        dimension: &str,
        entity: u32,
        kind: &str,
        pos: [i32; 3],
        from: &str,
        to: &str,
    );
}
