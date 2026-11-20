//! The `ender_chest_opened` hook (hooks/ender_chest_opened.rs): a guest's ender chest menu
//! opened. Called once `container_opened` let the menu open.
use super::Game;

impl Game<'_> {
    /// A guest opened the ender chest at `pos`; `player` is the guest's UUID. Server-side.
    #[inline(always)]
    pub(in crate::game) fn announce_ender_chest_opened(&self, player: u128, pos: [i32; 3]) {
        self.fire_ender_chest_opened(player, pos);
    }
}
