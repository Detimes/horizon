//! The `ender_chest_opened` hook (hooks/ender_chest_opened.rs): a guest's ender chest menu
//! opened. Called once `container_opened` let the menu open.
use super::Game;

impl Game<'_> {
    /// A guest opened the ender chest at `pos`; `player` is the guest's UUID. Server-side.
    #[inline(always)]
    pub(in crate::game) fn native_ender_chest_opened(&mut self, player: u128, pos: [i32; 3]) {
        mod_api::dispatch_ender_chest_opened(self, player, pos);
    }
}
