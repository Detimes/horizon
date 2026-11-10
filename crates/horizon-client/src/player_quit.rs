//! Why a player left the server, as a native mod hears it (`player_logged_out_v2`).
use std::sync::OnceLock;

/// Why a player left the server, for `player_logged_out_v2` (`native_player_logged_out` says only that they did).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeQuitReason {
    /// A clean disconnect: the client quit, its connection was reset, or it was moved to another server.
    Disconnected,
    /// The server kicked them, with the kick's text as the client shows it (`native_kick`, `/kick`, a ban, the
    /// packet-rate limit).
    Kicked(String),
    /// The keep-alive timeout: the client stopped answering.
    TimedOut,
    /// The server is stopping, and the player is still listed.
    ServerStopping,
    /// The connection failed: a malformed packet or a read error, with its text.
    Error(String),
}

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
