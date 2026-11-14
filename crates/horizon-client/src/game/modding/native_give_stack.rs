//! Whole stacks for server mods: one given to a player as `/give` gives it
//! (`player_give_stack`), and one read from SNBT as a saved item is (`item_from_snbt`).
use super::Game;
use horizon_client::{items::ItemStack, lan::playerdata};
use mod_api::PlayerRef;

/// The most SNBT text `item_from_snbt` reads: `NbtAccounter`'s default quota, 2 MiB.
const MAX_SNBT_BYTES: usize = 2 * 1024 * 1024;

impl Game<'_> {
    /// `GiveCommand.giveItem` for one stack: `Inventory.add`, the rest thrown for the player
    /// alone to pick up, in the player's turn. Nothing given for a dead, stale or unavailable
    /// player and on a client.
    pub(super) fn player_give_stack(&mut self, player: PlayerRef, stack: ItemStack) -> bool {
        if self.remote.is_some() {
            return false;
        }
        let owner = Some(playerdata::uuid_ints(player.uuid()));
        self.native_as_player_handle(player, |game| {
            game.give_acting_stack(stack, owner);
            // containerMenu.broadcastChanges: InventoryChangeTrigger.
            if game.player.is_some() {
                game.check_inventory_trigger();
            }
        })
    }

    /// `ItemStack.CODEC` reading the compound `snbt` stands for (`TagParser`): only that
    /// compound and whitespace, at most 2 MiB (the parser stops at 512 levels).
    pub(super) fn item_from_snbt(&self, snbt: &str) -> Option<ItemStack> {
        if snbt.len() > MAX_SNBT_BYTES {
            return None;
        }
        let (tag, end) = horizon_core::nbt::parse_snbt(snbt).ok()?;
        if !snbt.as_bytes()[end..].iter().all(u8::is_ascii_whitespace) {
            return None;
        }
        let stack = horizon_items::item_nbt::item_from_nbt(&tag)?;
        self.items().get(&stack.item).is_some().then_some(stack)
    }
}
