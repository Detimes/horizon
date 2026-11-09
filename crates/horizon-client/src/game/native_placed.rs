//! A block a server mod writes as a player placed it (docs/native-mods.md,
//! API @API:place_block_as@): the placed-by rules and the neighbours' shapes
//! of `BlockItem.place`, with no item stack, sound or game event. Axiom's
//! single block placement needs them; a mod that sets blocks with
//! `native_set_block` gets the neighbours told but not `setPlacedBy`.
use super::Game;
use horizon_core::position::BlockPos;
use horizon_world::registry::BlockStateId;

impl Game<'_> {
    /// Writes `state` at `pos` in the level in place, `dimension`, as the
    /// server player `player` placed it (`Behaviours::place_block_state`):
    /// `Block.setPlacedBy` runs (a bed places its head, a door its upper
    /// half), the neighbours hear of the block and, unless
    /// `update_neighbour_shapes` is false, their shapes update (fences, walls
    /// and stairs connect). The state is written as given. A block entity
    /// block gets its block entity. False, with nothing written, for a player
    /// not on the server or dead, a dimension other than the level in place,
    /// an unknown state, a position outside the world, and on a client's copy
    /// of the world. API @API:place_block_as@.
    pub(super) fn native_place_block_as(
        &mut self,
        player: u128,
        dimension: &str,
        pos: [i32; 3],
        state: BlockStateId,
        update_neighbour_shapes: bool,
    ) -> bool {
        let mut placed = false;
        // The write runs with that player's state in place, as their own
        // placement would; the player must be on the server.
        let ran = self.native_as_player(player, |game| {
            placed = game
                .native_write_placed_block(dimension, pos, state, update_neighbour_shapes)
                .is_some();
        });
        ran && placed
    }

    fn native_write_placed_block(
        &mut self,
        dimension: &str,
        pos: [i32; 3],
        state: BlockStateId,
        update_neighbour_shapes: bool,
    ) -> Option<BlockStateId> {
        if (self.serving.is_none() && self.options.dedicated.is_none())
            || dimension != self.dim.id()
            || self.pack.assets.registry.state(state).is_none()
        {
            return None;
        }
        self.ensure_stateful_entity_table();
        let at = BlockPos::new(pos[0], pos[1], pos[2]);
        let placed = self.with_level(|b, level, ticks| {
            b.place_block_state(level, ticks, at, state, update_neighbour_shapes)
        })?;
        // As `native_set_block` and the player's placement: the block entity
        // a new block brings.
        let key = (pos[0], pos[1], pos[2]);
        if !self.world.block_entities.stateful_entity(placed.0)
            && let Some(entity) = self.new_block_entity_for_state(placed)
        {
            self.world.block_entities.insert(key, entity);
        }
        let _ = self.process_level_events();
        Some(placed)
    }
}
