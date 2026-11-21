//! The `portal_create` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0321

pub trait Hooks {
    /// A Nether portal was built for a dimension travel, or an End platform
    /// was made for an arrival. `pos` is the lower corner of the Nether
    /// portal rectangle or the End platform origin. `created` lists the
    /// Nether portal blocks of a travel, in order (empty for an End platform).
    /// The completed blocks are already in the world.
    #[notes = r#"## Portal creation

`portal_create` is told after a Nether portal is built for a dimension travel, or an End
platform for an arrival. `created` lists the portal blocks of a travel, from `pos` along the
travel axis and then up, row by row. It does not list the obsidian frame, and an End platform
has an empty list for now. Portals lit by fire (`portal_spawn`) do not fire this hook yet.
The creating entity is not reported yet."#]
    #[hook(
        side = "server",
        contract = "listen",
        neoforge = "none (NeoForge has no such event)"
    )]
    fn portal_create();
}
