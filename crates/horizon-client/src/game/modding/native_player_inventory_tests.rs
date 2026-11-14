//! `player_inventory_slots` and `player_set_inventory_slot` against real LAN guests, through the
//! public player handle.

use super::native_guest_turn_tests::with_guests;
use mod_api::{Host, ItemStack, StackWriteError, prelude::*};

fn api_item(game: &super::Game<'_>, snbt: &str) -> Option<ItemStack> {
    let host: &dyn Host = game;
    host.item_from_snbt(snbt)
}

fn stack(game: &super::Game<'_>, snbt: &str) -> ItemStack {
    api_item(game, snbt).unwrap_or_else(|| panic!("{snbt}"))
}

fn guest<'a>(game: &'a super::Game<'_>, conn: u32) -> &'a horizon_client::lan::guest::Guest {
    game.lan.as_ref().unwrap().hub.guest(conn).unwrap()
}

const BLADE: &str = r#"{id:"minecraft:diamond_sword",count:1,components:{"minecraft:damage":5,"minecraft:custom_name":"Blade","minecraft:custom_data":{husksync:1b}}}"#;

/// Slots go by `Inventory`'s numbers both ways, out of the player's turn and in it; the
/// inventory menu has the changed slots left to send, and a stack with components comes back
/// as it went in.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn inventory_slots_are_read_and_written_by_vanillas_numbers() {
    with_guests(|game| {
        let (a, b) = {
            let players = game.players().server_players();
            (players[0], players[1])
        };
        let blade = stack(game, BLADE);
        let boots = stack(game, r#"{id:"minecraft:iron_boots",count:1}"#);
        let shield = stack(game, r#"{id:"minecraft:shield",count:1}"#);
        assert_eq!(b.inventory_slots(game), Some(Vec::new()));
        assert_eq!(b.set_inventory_slot(game, 8, Some(blade.clone())), Ok(true));
        assert_eq!(
            b.set_inventory_slot(game, 36, Some(boots.clone())),
            Ok(true)
        );
        assert_eq!(
            b.set_inventory_slot(game, 40, Some(shield.clone())),
            Ok(true)
        );
        let g = guest(game, 13);
        assert_eq!(
            g.inventory[8].as_ref().map(|s| s.item.as_ref()),
            Some("minecraft:diamond_sword")
        );
        assert_eq!(
            g.armor[3].as_ref().map(|s| s.item.as_ref()),
            Some("minecraft:iron_boots"),
            "36 is the feet"
        );
        assert_eq!(
            g.offhand.as_ref().map(|s| s.item.as_ref()),
            Some("minecraft:shield")
        );
        // The inventory menu's hotbar 8 is slot 44, feet 8, off hand 45: unsent yet.
        for menu in [44, 8, 45] {
            assert_ne!(g.menu_slot(menu), g.menu_sent[menu], "menu slot {menu}");
        }
        assert_eq!(
            b.inventory_slots(game),
            Some(vec![(8, blade.clone()), (36, boots), (40, shield)])
        );
        assert_eq!(
            api_item(game, BLADE).as_ref(),
            Some(&blade),
            "damage, name and custom data survive the round trip"
        );
        // In another player's turn the first one is displaced, and still reached.
        game.as_guest(12, |game| {
            assert_eq!(
                a.set_inventory_slot(game, 39, Some(blade.clone())),
                Ok(true)
            );
            assert_eq!(a.inventory_slots(game), Some(vec![(39, blade.clone())]));
            assert_eq!(b.set_inventory_slot(game, 8, None), Ok(true));
        })
        .unwrap();
        assert_eq!(guest(game, 13).inventory[8], None);
        assert_eq!(
            a.inventory_slots(game),
            Some(vec![(39, blade)]),
            "39 is the head"
        );
    });
}

/// This API refuses a slot past 42, a dead player and a stack the item does not allow, and
/// writes nothing for any of them.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_refused_inventory_write_changes_nothing() {
    with_guests(|game| {
        let b = game.players().server_players()[1];
        let rock = stack(game, r#"{id:"minecraft:stone",count:1}"#);
        assert_eq!(
            b.set_inventory_slot(game, 43, Some(rock.clone())),
            Ok(false)
        );
        assert_eq!(
            b.set_inventory_slot(game, u32::MAX, Some(rock.clone())),
            Ok(false)
        );
        let mut too_many = rock.clone();
        too_many.set_count(65).unwrap();
        assert_eq!(
            b.set_inventory_slot(game, 0, Some(too_many)),
            Err(StackWriteError::CountExceeded { maximum: 64 })
        );
        assert_eq!(b.inventory_slots(game), Some(Vec::new()));
        game.lan
            .as_mut()
            .unwrap()
            .hub
            .guests
            .update_connection(13, |g| g.dead = true);
        assert_eq!(b.set_inventory_slot(game, 0, Some(rock)), Ok(false));
        assert_eq!(
            b.inventory_slots(game),
            Some(Vec::new()),
            "a dead player is read, not written"
        );
    });
}
