//! Several dragons in one level: each ticks on its own and parts resolve to their owner.
use super::*;
use crate::game::native_entity_refs::tests::with_game;

fn two_dragons(game: &mut Game<'_>) {
    game.dragon.clear();
    for (id, at) in [
        (20, Vec3::new(0.0, 80.0, 0.0)),
        (40, Vec3::new(100.0, 80.0, 100.0)),
    ] {
        let mut d = Dragon::new(at, 0.0, 0);
        d.id = id;
        game.dragon.push(d);
    }
}

#[test]
#[ignore = "daily"]
fn every_dragon_ticks_once_and_keeps_its_order() {
    with_game(|game| {
        two_dragons(game);
        game.tick_dragons();
        let ticks: Vec<_> = game.dragon.iter().map(|d| (d.id, d.ticks)).collect();
        assert_eq!(ticks, [(20, 1), (40, 1)]);
        game.tick_dragons();
        let ticks: Vec<_> = game.dragon.iter().map(|d| (d.id, d.ticks)).collect();
        assert_eq!(ticks, [(20, 2), (40, 2)]);
    });
}

#[test]
#[ignore = "daily"]
fn a_part_id_resolves_to_its_own_dragon() {
    with_game(|game| {
        two_dragons(game);
        game.tick_dragons();
        // Part 1 of each dragon follows its own owner, 100 blocks apart.
        let (first, _) = game.dragon_part(21).unwrap();
        let (second, _) = game.dragon_part(41).unwrap();
        assert!(first.x < 50.0, "{first:?}");
        assert!(second.x > 50.0, "{second:?}");
        assert_eq!(game.tracked_special_box(41).is_some(), true);
    });
}

#[test]
#[ignore = "daily"]
fn killing_one_dragon_leaves_the_other() {
    with_game(|game| {
        two_dragons(game);
        game.kill_dragon(40);
        assert_eq!(game.dragon.iter().map(|d| d.id).collect::<Vec<_>>(), [20]);
        game.tick_dragons();
        assert_eq!(game.dragon.get(20).unwrap().ticks, 1);
    });
}


#[test]
#[ignore = "daily"]
fn the_fight_follows_its_dragon_by_uuid() {
    with_game(|game| {
        two_dragons(game);
        assert_eq!(game.fight_dragon_index(), None);
        game.fight.dragon_uuid = Some(game.uuid_for(40));
        assert_eq!(game.fight_dragon_index(), Some(1));
        // The fight's health bar reads its own dragon, not the first one.
        game.dragon.get_mut(20).unwrap().health = 10.0;
        game.fight.loaded = true;
        game.fight.killed = false;
        if game.dimension_type().has_ender_dragon_fight {
            assert_eq!(game.dragon_boss_progress([0.0, 128.0, 0.0], true), Some(1.0));
        }
        // Killing an extra dragon leaves the fight's.
        game.kill_dragon(20);
        assert_eq!(game.fight_dragon_index(), Some(0));
        assert_eq!(game.dragon.iter().map(|d| d.id).collect::<Vec<_>>(), [40]);
    });
}

#[test]
#[ignore = "daily"]
fn a_saved_dragon_is_not_loaded_twice_but_two_different_ones_are() {
    with_game(|game| {
        game.dragon.clear();
        let a = Dragon::new(Vec3::new(0.0, 80.0, 0.0), 0.0, 0).to_nbt([1, 0, 0, 1], None);
        let b = Dragon::new(Vec3::new(9.0, 80.0, 0.0), 0.0, 0).to_nbt([2, 0, 0, 2], None);
        game.load_dragon(a.clone(), Some([1, 0, 0, 1]));
        game.load_dragon(a, Some([1, 0, 0, 1]));
        game.load_dragon(b, Some([2, 0, 0, 2]));
        assert_eq!(game.dragon.iter().count(), 2);
        let chunks = game.entity_chunks(&[(0, 0)].into_iter().collect(), false);
        assert_eq!(chunks.iter().map(|c| c.1.len()).sum::<usize>(), 2);
    });
}
