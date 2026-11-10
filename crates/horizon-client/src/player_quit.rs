//! Why a player left the server, as a native mod hears it (`player_logged_out_v2`).
use crate::mod_api::NativeQuitReason;
use std::sync::OnceLock;

/// The mods' listener: the executable installs it (`entry.rs`) before any player can leave.
type Listener = fn(u128, &str, NativeQuitReason);

static LISTENER: OnceLock<Listener> = OnceLock::new();

/// Installs the listener once; a second installation keeps the first.
pub fn install(listener: Listener) {
    let _ = LISTENER.set(listener);
}

fn player_logged_out_v2(player: u128, name: &str, reason: NativeQuitReason) {
    if let Some(listener) = LISTENER.get() {
        listener(player, name, reason);
    }
}

/// `player` (`name`) left the server for `reason` (NeoForge's `PlayerLoggedOutEvent` has none). Heard right
/// after `native_player_logged_out`, for every player that one hears of. Server-side. API @API:player_logged_out_v2@.
#[inline(never)]
pub fn native_player_logged_out_v2(player: u128, name: &str, reason: NativeQuitReason) {
    player_logged_out_v2(player, name, reason);
}
