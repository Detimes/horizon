//! `player_leaving`: the mods hear a placed network player before the game removes and saves them.
use super::Game;

impl Game<'_> {
    /// Tells the mods that the network player on connection `conn` (`uuid`, `name`) is about to
    /// leave with the plain-text `reason`, with that player's state swapped in: they are still
    /// listed and placed, before their menus close and their file is saved. Server-side.
    pub(in crate::game) fn announce_player_leaving(
        &mut self,
        conn: u32,
        uuid: u128,
        name: &str,
        reason: &str,
    ) {
        self.as_guest(conn, |game| {
            game.native_player_leaving(uuid, name, reason);
        });
    }
}
