//! A block written as a player placed it (`Behaviours::place_block_state`):
//! `Block.setPlacedBy` runs and the neighbours hear of it, their shapes too
//! unless the caller turns that off.
use super::*;

fn place_state(
    w: &mut World,
    pos: BlockPos,
    state: BlockStateId,
    update_neighbour_shapes: bool,
) -> Option<BlockStateId> {
    w.f.behaviours.place_block_state(
        &mut w.level,
        &mut w.ticks,
        pos,
        state,
        update_neighbour_shapes,
    )
}

#[test]
fn a_bed_written_as_placed_places_its_head() {
    let mut w = world!();
    w.floor(0, 6, "grass_block");
    let foot = w.id(
        "red_bed",
        &[("facing", "east"), ("occupied", "false"), ("part", "foot")],
    );
    let pos = BlockPos::new(0, 1, 0);
    assert!(place_state(&mut w, pos, foot, true).is_some());
    assert_eq!(w.prop(pos, "part"), "foot");
    assert_eq!(
        w.prop(BlockPos::new(1, 1, 0), "part"),
        "head",
        "the head is in the cell the bed faces"
    );
    assert_eq!(w.prop(BlockPos::new(1, 1, 0), "facing"), "east");
}

#[test]
fn a_door_written_as_placed_places_its_upper_half() {
    let mut w = world!();
    w.floor(0, 6, "grass_block");
    let lower = w.id(
        "oak_door",
        &[
            ("facing", "north"),
            ("half", "lower"),
            ("hinge", "left"),
            ("open", "false"),
            ("powered", "false"),
        ],
    );
    let pos = BlockPos::new(0, 1, 0);
    assert!(place_state(&mut w, pos, lower, true).is_some());
    assert_eq!(w.prop(pos, "half"), "lower");
    assert_eq!(w.prop(BlockPos::new(0, 2, 0), "half"), "upper");
}

#[test]
fn a_fence_written_as_placed_connects_its_neighbour() {
    let mut w = world!();
    w.floor(0, 6, "grass_block");
    let fence = w.id(
        "oak_fence",
        &[
            ("north", "false"),
            ("east", "false"),
            ("south", "false"),
            ("west", "false"),
            ("waterlogged", "false"),
        ],
    );
    let neighbour = BlockPos::new(1, 1, 0);
    w.set(neighbour, fence);
    assert_eq!(w.prop(neighbour, "west"), "false");
    let connected = w.id(
        "oak_fence",
        &[
            ("north", "false"),
            ("east", "true"),
            ("south", "false"),
            ("west", "false"),
            ("waterlogged", "false"),
        ],
    );
    let pos = BlockPos::new(0, 1, 0);
    assert!(place_state(&mut w, pos, connected, true).is_some());
    assert_eq!(
        w.prop(neighbour, "west"),
        "true",
        "the neighbour's shape updates to the placed fence"
    );
    assert_eq!(w.get(pos), connected, "the state written is the one given");
}

#[test]
fn without_shape_updates_the_neighbour_keeps_its_shape() {
    let mut w = world!();
    w.floor(0, 6, "grass_block");
    let fence = w.id(
        "oak_fence",
        &[
            ("north", "false"),
            ("east", "false"),
            ("south", "false"),
            ("west", "false"),
            ("waterlogged", "false"),
        ],
    );
    let neighbour = BlockPos::new(1, 1, 0);
    w.set(neighbour, fence);
    let pos = BlockPos::new(0, 1, 0);
    assert!(place_state(&mut w, pos, fence, false).is_some());
    assert_eq!(w.prop(neighbour, "west"), "false");
}

#[test]
fn nothing_is_written_outside_the_world() {
    let mut w = world!();
    let stone = w.id("stone", &[]);
    assert_eq!(
        place_state(&mut w, BlockPos::new(0, 1000, 0), stone, true),
        None
    );
}
