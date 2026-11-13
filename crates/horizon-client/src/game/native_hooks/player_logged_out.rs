use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// `player` (`name`) left the game (NeoForge's `PlayerLoggedOutEvent`), by
    /// any path: a lost connection, a kick, the server stopping or failing.
    /// Once for each player `native_player_logged_in` told of. They've already
    /// left `native_server_players`, except when the server stops, which tells
    /// before it saves, with them still listed and able to get a last message.
    /// `reason` is what ended the connection and `message` the console's "lost
    /// connection" line. Server-side.
    pub(in crate::game) fn native_player_logged_out(
        &mut self,
        player: u128,
        name: &str,
        reason: mod_api::QuitReason,
        message: &str,
    ) {
        self.fire_player_logged_out(player, name, reason, message);
    }

    /// A guest the hub removed left the game, for the cause it kept.
    pub(in crate::game) fn native_guest_logged_out(
        &mut self,
        guest: &horizon_client::lan::guest::Guest,
    ) {
        let reason = guest.leave_cause().quit_reason();
        self.native_player_logged_out(guest.uuid, &guest.name, reason, &guest.left_reason);
    }

    /// The server stops with `player` still on.
    pub(in crate::game) fn native_server_stopped_for(&mut self, player: u128, name: &str) {
        let reason = mod_api::QuitReason::ServerStopping;
        self.native_player_logged_out(player, name, reason, "Server closed");
    }
}
