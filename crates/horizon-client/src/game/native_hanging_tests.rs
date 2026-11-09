//! `native_set_hanging_facing` and `native_set_item_frame_rotation`: a painting or item frame turns and
//! its bounds follow, an item frame's item turns, and what is refused. The `Game` tests need prepared
//! resources (HORIZON_TEST_RESOURCES), as the other native tests do.
use super::native_summon_tests::game;
use horizon_client::entities::{ItemFrame, Painting};
use horizon_core::random::LegacyRandom;

fn frame(id: u32, facing: u8) -> ItemFrame {
    ItemFrame {
        id,
        pos: [4, 200, 4],
        facing,
        glow: false,
        item: None,
        rotation: 0,
        drop_chance: 1.0,
        fixed: false,
        invisible: false,
        check_interval: 0,
        custom_name: None,
        random: LegacyRandom::new(0),
    }
}

fn painting(id: u32, facing: u8) -> Painting {
    Painting {
        id,
        pos: [4, 200, 4],
        facing,
        variant: "minecraft:kebab".into(),
        width: 1,
        height: 1,
        check_interval: 0,
    }
}

#[test]
fn a_painting_takes_only_the_horizontal_facings_in_its_2d_value() {
    // Direction.get3DDataValue to get2DDataValue: south 3 to 0, west 4 to 1, north 2 to 2, east 5 to 3.
    let kept: Vec<Option<u8>> = (0..=5).map(super::painting_facing_2d).collect();
    assert_eq!(kept, [None, None, Some(2), Some(0), Some(1), Some(3)]);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn an_item_frame_turns_and_its_bounds_follow() {
    let mut game = game();
    game.item_frames.push(frame(900, 3));
    // South: thin on z. West: thin on x. Up: thin on y.
    let thin = |game: &super::super::Game<'_>| {
        let (lo, hi) = game.item_frames[0].bounds();
        (0..3).map(|a| hi[a] - lo[a] < 0.1).collect::<Vec<_>>()
    };
    assert_eq!(thin(&game), [false, false, true]);
    assert!(game.native_set_hanging_facing(900, 4));
    assert_eq!(game.item_frames[0].facing, 4);
    assert_eq!(thin(&game), [true, false, false]);
    assert!(game.native_set_hanging_facing(900, 1));
    assert_eq!(thin(&game), [false, true, false]);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_painting_turns_to_a_horizontal_facing_only() {
    let mut game = game();
    game.paintings.push(painting(901, 0));
    assert!(game.native_set_hanging_facing(901, 5));
    assert_eq!(game.paintings[0].facing, 3);
    assert!(!game.native_set_hanging_facing(901, 1));
    assert!(!game.native_set_hanging_facing(901, 6));
    assert_eq!(game.paintings[0].facing, 3);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn an_unknown_entity_is_refused() {
    let mut game = game();
    game.item_frames.push(frame(900, 3));
    assert!(!game.native_set_hanging_facing(777, 2));
    assert!(!game.native_set_item_frame_rotation(777, 2));
    assert_eq!(game.item_frames[0].facing, 3);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn an_item_frame_rotation_is_written_and_eight_is_refused() {
    let mut game = game();
    game.item_frames.push(frame(902, 3));
    assert!(game.native_set_item_frame_rotation(902, 5));
    assert_eq!(game.item_frames[0].rotation, 5);
    assert!(!game.native_set_item_frame_rotation(902, 8));
    assert_eq!(game.item_frames[0].rotation, 5);
    assert!(game.native_set_item_frame_rotation(902, 7));
    assert_eq!(game.item_frames[0].rotation, 7);
}
