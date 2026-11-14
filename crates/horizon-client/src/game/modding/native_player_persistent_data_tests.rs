//! `player_persistent_data` and `player_set_persistent_data` against real LAN guests, through the
//! public player handle.

use super::native_guest_turn_tests::with_guests;
use horizon_core::nbt::{Compound, Tag};
use mod_api::prelude::*;

fn saved_data(game: &super::Game<'_>, conn: u32) -> Option<Tag> {
    let hub = &game.lan.as_ref().unwrap().hub;
    hub.guest(conn)?
        .saved
        .as_ref()?
        .get("NeoForgeData")
        .cloned()
}

/// A chain of compounds `levels` deep, each holding the next under `k`.
fn nested(levels: usize) -> Compound {
    let mut data = Compound::new();
    for _ in 1..levels {
        let mut outer = Compound::new();
        outer.insert("k".into(), Tag::Compound(data));
        data = outer;
    }
    data
}

/// The mods' compound goes into the player's own saved record, in their turn or out of it,
/// and an empty one takes the key away.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn persistent_data_is_kept_in_the_players_record() {
    with_guests(|game| {
        let players = game.players().server_players();
        let (a, b) = (players[0], players[1]);
        assert_eq!(b.persistent_data(game), Some(Compound::new()));
        let mut data = Compound::new();
        data.insert("husksync:key".into(), Tag::Int(7));
        assert!(b.set_persistent_data(game, data.clone()));
        assert_eq!(saved_data(game, 13), Some(Tag::Compound(data.clone())));
        assert_eq!(saved_data(game, 12), None);
        game.as_guest(12, |game| {
            assert_eq!(b.persistent_data(game), Some(data.clone()));
            assert!(a.set_persistent_data(game, data.clone()));
            assert_eq!(a.persistent_data(game), Some(data.clone()));
        })
        .unwrap();
        assert_eq!(saved_data(game, 12), Some(Tag::Compound(data)));
        assert!(b.set_persistent_data(game, Compound::new()));
        assert_eq!(saved_data(game, 13), None);
        assert_eq!(b.persistent_data(game), Some(Compound::new()));
    });
}

/// This API refuses data that would sit 512 levels deep or more in the player's file, and a
/// dead player, and keeps what was there.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn data_too_deep_or_for_a_dead_player_is_refused() {
    with_guests(|game| {
        let b = game.players().server_players()[1];
        let mut kept = Compound::new();
        kept.insert("kept".into(), Tag::Byte(1));
        assert!(b.set_persistent_data(game, kept.clone()));
        assert!(
            b.set_persistent_data(game, nested(511)),
            "511 levels are written"
        );
        assert_eq!(b.persistent_data(game), Some(nested(511)));
        assert!(b.set_persistent_data(game, kept.clone()));
        assert!(
            !b.set_persistent_data(game, nested(512)),
            "512 levels are not"
        );
        assert_eq!(
            b.persistent_data(game),
            Some(kept.clone()),
            "nothing was written"
        );
        game.lan
            .as_mut()
            .unwrap()
            .hub
            .guests
            .update_connection(13, |g| g.dead = true);
        assert!(!b.set_persistent_data(game, Compound::new()));
        assert_eq!(
            b.persistent_data(game),
            Some(kept),
            "a dead player is read, not written"
        );
    });
}
