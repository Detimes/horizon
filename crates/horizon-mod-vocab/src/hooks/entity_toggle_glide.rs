//! The `entity_toggle_glide` hook: declarations the mod API generator reads (see `hooks/mod.rs`).
// catalog-order: 0209

pub trait Hooks {
    /// A guest's fall flying state is about to change on the server, before
    /// it is applied (Paper `EntityToggleGlideEvent`). `gliding` is the new
    /// state. Set `cancel` to keep the guest's current state: a refused start
    /// leaves them not gliding, a refused stop leaves them gliding.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        cancellable = true,
        neoforge = "none (NeoForge has no such event)"
    )]
    fn entity_toggle_glide();
}
