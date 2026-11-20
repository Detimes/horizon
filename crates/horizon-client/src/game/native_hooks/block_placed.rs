use super::*;

impl Game<'_> {
    /// Notify after a player places a block, with the replaced state
    pub(in crate::game) fn native_block_placed(&mut self, player: Option<PlayerRef>, dimension: &str, pos: [i32; 3], placed: BlockState, replaced: BlockState) {
        self.fire_block_placed(player, dimension, pos, placed, replaced);
    }
}
