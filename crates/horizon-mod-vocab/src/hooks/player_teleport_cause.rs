//! The `player_teleport_cause` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A server player was teleported from `from` to `to`, now in `dimension` (Bukkit's
    /// `PlayerTeleportEvent.getCause()`), after the move. `cause` is `command` (`/tp`, `/teleport`)
    /// or `spectate` (a spectator's teleport to a player). Ender pearls, chorus fruit, portals and
    /// end gateways have their own hooks and are not heard here yet. Heard in addition to
    /// `player_teleported`, which keeps its published signature (#19020), with the same cases
    /// plus the spectator's. Listen-only. Server-side.
    #[hook(
        side = "server",
        contract = "listen",
        neoforge = "none (EntityTeleportEvent names no cause)",
        paper = "PlayerTeleportEvent"
    )]
    fn player_teleport_cause(
        player: u128,
        name: &str,
        from: [f64; 3],
        to: [f64; 3],
        dimension: &str,
        cause: &str,
    );
}
