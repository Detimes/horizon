//! The `dragon_change_phase` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The Ender Dragon is about to change phase, from `current` to `new_phase`
    /// (`EnderDragonPhaseManager.setPhase`). Set `new_phase` to another phase to
    /// have the dragon begin that one instead, or set `cancel` to keep the
    /// current phase. Heard for the changes the game decides itself (landing,
    /// strafing a player, taking off when hurt on the fountain, the dying
    /// flight) and for a mod's own `set_phase`; not for a phase read from a
    /// save, and not when the dragon already holds the phase it is asked for.
    /// A replacement equal to `current` leaves the dragon as it is. Server-side.
    #[notes = r#"## Dragon change phase

- **`dragon_change_phase(host, event)`**: the moment before the Ender Dragon moves from `event.current` to `event.new_phase`, as Paper's `EnderDragonChangePhaseEvent` fires (`getCurrentPhase`, `getNewPhase`, `setNewPhase`, `setCancelled`). A fight mod locks a cinematic dragon in `Hover` by cancelling every other phase, announces `Dying`, or turns a landing into a strafing run by writing another phase to `event.new_phase`. The dragon's own tick also asks here: a cancelled `LandingApproach` leaves it circling and it asks again when its rules want to land. A phase the dragon already holds is not a change and is not heard (a strafing dragon asked to strafe keeps its player)."#]
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        neoforge = "none (NeoForge has no such event)",
        paper = "EnderDragonChangePhaseEvent",
        cancellable = true
    )]
    fn dragon_change_phase();
}
