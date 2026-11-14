//! A server player's inventory by vanilla's slot numbers (`Inventory.getItem`/`setItem`), for
//! mods that copy or edit it whole: player data sync, `/invsee`.
use super::{Game, commands::items::Slot, native_server::PlayerRead};
use horizon_client::items::ItemStack;
use horizon_items::equipment::{BODY, Equipment, SADDLE};
use mod_api::PlayerRef;

/// `Inventory`'s slot numbers past the 36 main slots (`EQUIPMENT_SLOT_MAPPING`).
const SLOTS: u32 = 43;

/// The slot the game keeps `Inventory` slot `slot` in.
fn inventory_slot(slot: u32) -> Option<Slot> {
    Some(match slot {
        0..36 => Slot::Inv(slot as usize),
        // `armor` is head first: 36 feet is its last.
        36..40 => Slot::Armor((39 - slot) as usize),
        40 => Slot::Offhand,
        41 => Slot::Equipment(BODY),
        42 => Slot::Equipment(SADDLE),
        _ => return None,
    })
}

/// The non-empty slots, by number, of an inventory laid out as the game keeps it.
fn numbered(
    inventory: &[Option<ItemStack>],
    armor: &[Option<ItemStack>; 4],
    offhand: &Option<ItemStack>,
    equipment: &Equipment,
) -> Vec<(u32, ItemStack)> {
    (0..SLOTS)
        .filter_map(|slot| {
            let stack = match inventory_slot(slot)? {
                Slot::Inv(i) => inventory.get(i).cloned().flatten(),
                Slot::Armor(i) => armor[i].clone(),
                Slot::Offhand => offhand.clone(),
                Slot::Equipment(i) => equipment.get(i).cloned(),
                _ => None,
            };
            Some((slot, stack.filter(|s| s.count > 0)?))
        })
        .collect()
}

impl Game<'_> {
    /// `Inventory.getItem` for every slot that holds something.
    pub(super) fn player_inventory_slots(&self, player: PlayerRef) -> Option<Vec<(u32, ItemStack)>> {
        if self.remote.is_some() {
            return None;
        }
        Some(match self.player_handle_read(player)? {
            PlayerRead::Acting { .. } => {
                numbered(&self.inventory, &self.armor, &self.offhand, &self.equipment)
            }
            PlayerRead::Guest(g) => numbered(&g.inventory, &g.armor, &g.offhand, &g.equipment),
        })
    }

    /// `Inventory.setItem`, in the player's turn: the tick's inventory sync tells the client.
    pub(super) fn player_set_inventory_slot(
        &mut self,
        player: PlayerRef,
        slot: u32,
        stack: Option<ItemStack>,
    ) -> bool {
        let Some(slot) = inventory_slot(slot).filter(|_| self.remote.is_none()) else {
            return false;
        };
        let mut set = false;
        let ran = self.native_as_player_handle(player, |game| {
            set = game.slot_set(&slot, stack.filter(|s| s.count > 0));
            if set && game.player.is_some() {
                game.check_inventory_trigger();
            }
        });
        ran && set
    }
}

