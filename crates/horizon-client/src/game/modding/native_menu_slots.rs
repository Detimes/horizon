//! `player_open_menu_slots`: the container menu a guest has open, as the menu readers accept it.

use super::*;
use mod_api::{MenuRef, PlayerRef};

impl Game<'_> {
    /// `HumanEntity.getOpenInventory` for a guest: the menu it has open. A client's own menu is not lent here.
    pub(super) fn player_open_menu_slots(&self, player: PlayerRef) -> Option<MenuRef> {
        if self.remote.is_some() {
            return None;
        }
        let super::native_server::PlayerRead::Guest(guest) = self.player_handle_read(player)?
        else {
            return None;
        };
        // Only a menu the readers (`menu_slot`, `menu_carried`) accept is lent: a crafting table,
        // merchant, workstation, lectern or mount has no window of its own here and would read as
        // window 0, which they refuse.
        let window = guest.menu.as_ref()?.window;
        let menu = mod_api::__private::game::menu_ref(player, window, guest.menu_generation);
        self.menu_container_id(menu).map(|_| menu)
    }
}
