//! `player_ender_chest` and `player_set_ender_chest_slot` against real LAN guests, through the
//! public player handle.

use super::native_guest_turn_tests::with_guests;
use mod_api::{Host, ItemStack, StackWriteError, prelude::*};

fn api_item(game: &super::Game<'_>, snbt: &str) -> Option<ItemStack> {
    let host: &dyn Host = game;
    host.item_from_snbt(snbt)
}

const BOOK: &str = r#"{id:"minecraft:written_book",count:1,components:{"minecraft:written_book_content":{title:{raw:"Log"},author:"guest13",pages:[{raw:"one"}]}}}"#;

/// Ender chest slots are written and read back by number, out of the player's turn and in
/// it, components and all, and each player has their own.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn ender_chest_slots_are_read_and_written() {
    with_guests(|game| {
        let players = game.players().server_players();
        let (a, b) = (players[0], players[1]);
        let book = api_item(game, BOOK).unwrap();
        let pearls = api_item(game, r#"{id:"minecraft:ender_pearl",count:16}"#).unwrap();
        assert_eq!(b.ender_chest(game), Some(Vec::new()));
        assert_eq!(
            b.set_ender_chest_slot(game, 26, Some(book.clone())),
            Ok(true)
        );
        game.as_guest(12, |game| {
            assert_eq!(
                a.set_ender_chest_slot(game, 0, Some(pearls.clone())),
                Ok(true)
            );
            assert_eq!(a.ender_chest(game), Some(vec![(0, pearls.clone())]));
            assert_eq!(b.ender_chest(game), Some(vec![(26, book.clone())]));
        })
        .unwrap();
        assert_eq!(b.ender_chest(game), Some(vec![(26, book.clone())]));
        assert_eq!(a.ender_chest(game), Some(vec![(0, pearls)]));
        assert_eq!(b.set_ender_chest_slot(game, 26, None), Ok(true));
        assert_eq!(b.ender_chest(game), Some(Vec::new()));
    });
}

/// This API refuses a slot past 26, a dead player and a stack the item does not allow, and
/// writes nothing for any of them.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_refused_ender_chest_write_changes_nothing() {
    with_guests(|game| {
        let b = game.players().server_players()[1];
        let book = api_item(game, BOOK).unwrap();
        assert_eq!(
            b.set_ender_chest_slot(game, 27, Some(book.clone())),
            Ok(false)
        );
        assert_eq!(
            b.set_ender_chest_slot(game, u32::MAX, Some(book.clone())),
            Ok(false)
        );
        let mut too_many = api_item(game, r#"{id:"minecraft:diamond_sword"}"#).unwrap();
        too_many.set_count(2).unwrap();
        assert_eq!(
            b.set_ender_chest_slot(game, 0, Some(too_many)),
            Err(StackWriteError::CountExceeded { maximum: 1 })
        );
        assert_eq!(b.ender_chest(game), Some(Vec::new()));
        game.lan
            .as_mut()
            .unwrap()
            .hub
            .guests
            .update_connection(13, |g| g.dead = true);
        assert_eq!(b.set_ender_chest_slot(game, 0, Some(book)), Ok(false));
        assert_eq!(b.ender_chest(game), Some(Vec::new()));
    });
}
