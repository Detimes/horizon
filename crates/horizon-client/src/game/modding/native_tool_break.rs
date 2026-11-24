//! `tool_break_block`: the break as the acting player, with drops and tool damage.

use super::*;
use mod_api::PlayerRef;

impl Game<'_> {
    /// `Player#breakBlock` for the acting player. Other players are refused as `NotThisPlayer`.
    pub(super) fn tool_break_block(
        &mut self,
        player: PlayerRef,
        pos: [i32; 3],
    ) -> BlockBreakResult {
        // During a guest's turn the acting player is that guest, not the local client.
        let acting = self.native_acting_player().or_else(|| self.local_player());
        if acting != Some(player) {
            return BlockBreakResult::NotThisPlayer;
        }
        let before = self.world.block(pos[0], pos[1], pos[2]);
        match self.player_break_block(pos) {
            Ok(true) => BlockBreakResult::Broken,
            Ok(false) => BlockBreakResult::Kept,
            // An error after the block was removed (level events, say) still broke it.
            Err(_) if self.world.block(pos[0], pos[1], pos[2]) != before => {
                BlockBreakResult::Broken
            }
            Err(_) => BlockBreakResult::Kept,
        }
    }
}
